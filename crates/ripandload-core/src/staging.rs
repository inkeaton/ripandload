//! The Staging folder: what is waiting in it to be Uploaded, and clearing
//! it out afterwards.
//!
//! The Upload tab shows the Staging folder's music files grouped into Album
//! folders and, once a Comparison exists, how many of them an Upload would
//! still send. This module reads the folder for that, and deletes its
//! contents on request.
//!
//! This module runs no tools. A Comparison is built from the files that
//! Compare with Server's rsync dry run listed, which [`itemized_file`]
//! reads out of its output.
//!
//! [`itemized_file`]: crate::command::itemized_file

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::StagingError;

// == Music files ==============================================================

/// Extensions of the audio files streamrip writes, plus common ones from
/// other tools, for people who keep their library in the Staging folder.
const MUSIC_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "m4a", "mp4", "aac", "ogg", "opus", "wav", "aiff", "aif", "alac", "wv", "ape",
    "dsf", "dff",
];

/// Returns whether `path` is a music file, by its extension in any case.
///
/// Covers, lyrics and cue sheets aren't music files, though Uploads still
/// send them.
pub fn is_music(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            MUSIC_EXTENSIONS
                .iter()
                .any(|music| extension.eq_ignore_ascii_case(music))
        })
}

// == Staging contents =========================================================

/// The music files of one Album folder, or the single tracks.
///
/// The single tracks are the music files directly in the Staging folder,
/// shown as "Single tracks".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicGroup {
    /// The Album folder, relative to the Staging folder; `None` for the
    /// single tracks.
    pub folder: Option<PathBuf>,
    /// Its music files, relative to the Staging folder, sorted. An Album
    /// folder's include those in its disc folders.
    pub files: Vec<PathBuf>,
}

/// What's in the Staging folder: its music, grouped, and how many files in
/// all.
#[derive(Debug, Clone)]
pub struct StagingContents {
    /// The music files by Album folder, as [`music_groups`] returns them.
    pub groups: Vec<MusicGroup>,
    /// How many files there are in all, music or not.
    files: usize,
}

impl StagingContents {
    /// Reads `folder`, the Staging folder, as it is now.
    ///
    /// The music and the file count come from two walks of the folder, so
    /// files that change in between can make them disagree.
    ///
    /// # Errors
    ///
    /// The error from [`music_groups`] or [`count_files`] if a folder
    /// couldn't be listed.
    pub fn read(folder: &Path) -> Result<Self, StagingError> {
        Ok(Self {
            groups: music_groups(folder)?,
            files: count_files(folder)?,
        })
    }

    /// Returns every music file, relative to the Staging folder, group by
    /// group.
    pub fn music_files(&self) -> impl Iterator<Item = &Path> {
        self.groups
            .iter()
            .flat_map(|group| &group.files)
            .map(PathBuf::as_path)
    }

    /// Returns how many music files there are.
    pub fn music_count(&self) -> usize {
        self.groups.iter().map(|group| group.files.len()).sum()
    }

    /// Returns whether the Staging folder holds no files at all.
    pub fn is_empty(&self) -> bool {
        self.files == 0
    }

    /// Returns how many files aren't music files: covers, lyrics and the
    /// like.
    pub fn other_count(&self) -> usize {
        // The two counts come from two walks; files deleted in between could
        // make the music outnumber them.
        self.files.saturating_sub(self.music_count())
    }
}

/// Returns the music files in `folder`, grouped by Album folder.
///
/// A music file's Album folder is the folder it sits in, or the one above
/// if that's a disc folder. That holds whatever the folder Naming template
/// or streamrip's options make of the layout.
///
/// # Returns
///
/// One [`MusicGroup`] per Album folder, sorted by folder, then the single
/// tracks, if any. Nothing for a missing or musicless `folder`.
///
/// # Errors
///
/// [`StagingError::Read`] if `folder` or a folder below it couldn't be
/// listed.
pub fn music_groups(folder: &Path) -> Result<Vec<MusicGroup>, StagingError> {
    let mut files = Vec::new();
    collect_music(folder, folder, &mut files)?;

    // The empty folder, the Staging folder itself, holds the single tracks.
    let mut groups: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for file in files {
        groups.entry(album_folder(&file)).or_default().push(file);
    }
    let singles = groups.remove(Path::new(""));
    let mut groups: Vec<MusicGroup> = groups
        .into_iter()
        .map(|(folder, files)| MusicGroup {
            folder: Some(folder),
            files,
        })
        .collect();
    groups.extend(singles.map(|files| MusicGroup {
        folder: None,
        files,
    }));
    for group in &mut groups {
        group.files.sort();
    }
    Ok(groups)
}

/// Counts the files in `folder` and every folder below it.
///
/// Folders themselves don't count, and symlinks are counted, not followed.
///
/// # Returns
///
/// The number of files, 0 for a missing or empty `folder`.
///
/// # Errors
///
/// [`StagingError::Read`] if `folder` or a folder below it couldn't be
/// listed.
pub fn count_files(folder: &Path) -> Result<usize, StagingError> {
    let mut count = 0;
    for (path, is_dir) in entries(folder)? {
        count += if is_dir { count_files(&path)? } else { 1 };
    }
    Ok(count)
}

// == Comparisons ==============================================================

/// Which music files in the Staging folder are on the Server.
///
/// It is what a Compare with Server found, or what a successful Upload left:
/// one where the Server has everything.
#[derive(Debug, Clone)]
pub struct Comparison {
    /// The Staging folder compared.
    folder: PathBuf,
    /// The music files in the Staging folder at the time. A file that came
    /// later, from a Download or by hand, isn't known to be on the Server.
    music_then: BTreeSet<PathBuf>,
    /// What an Upload would have sent then, relative to `folder`.
    to_send: BTreeSet<PathBuf>,
    /// When the Comparison was made.
    pub at: Instant,
}

impl Comparison {
    /// A Comparison of the Staging folder as `contents` found it, made now.
    ///
    /// # Arguments
    ///
    /// - `folder`: the Staging folder compared.
    /// - `contents`: what was in `folder` at the time.
    /// - `to_send`: the files Compare with Server's rsync listed, relative to
    ///   `folder`; empty after an Upload that worked.
    pub fn new(folder: &Path, contents: &StagingContents, to_send: &[PathBuf]) -> Self {
        Self {
            folder: folder.to_owned(),
            music_then: contents.music_files().map(Path::to_path_buf).collect(),
            to_send: to_send.iter().cloned().collect(),
            at: Instant::now(),
        }
    }

    /// Returns whether the Comparison was made of `folder`.
    ///
    /// A Comparison of another Staging folder says nothing about this one.
    pub fn is_for(&self, folder: &Path) -> bool {
        self.folder == folder
    }

    /// Returns whether `file` is known to be on the Server.
    ///
    /// It is when it was in the Staging folder at the time and rsync didn't
    /// list it. A file that came later isn't, even if the Server has it.
    pub fn is_uploaded(&self, file: &Path) -> bool {
        self.music_then.contains(file) && !self.to_send.contains(file)
    }

    /// Returns how many of `files` an Upload would still send.
    ///
    /// That's every file not known to be on the Server (see
    /// [`Comparison::is_uploaded`]).
    pub fn to_upload<'a>(&self, files: impl IntoIterator<Item = &'a Path>) -> usize {
        files
            .into_iter()
            .filter(|file| !self.is_uploaded(file))
            .count()
    }
}

// == Clearing =================================================================

/// Deletes everything inside `folder`, the Staging folder, but not `folder`
/// itself.
///
/// A symlink is deleted, never what it points to.
///
/// # Returns
///
/// How many files were deleted, counted as [`count_files`] does. 0 for a
/// missing `folder`, which is left missing.
///
/// # Errors
///
/// - [`StagingError::Refused`] if `folder` is a filesystem root, an empty
///   path or the home folder, which the Staging folder was most likely set
///   to by mistake.
/// - [`StagingError::Read`] if a folder couldn't be listed.
/// - [`StagingError::Remove`] if a file or folder couldn't be deleted.
///   What was deleted before it stays deleted.
pub fn clear(folder: &Path) -> Result<usize, StagingError> {
    if is_unsafe_to_clear(folder) {
        return Err(StagingError::Refused {
            path: folder.to_owned(),
        });
    }
    let count = count_files(folder)?;
    for (path, is_dir) in entries(folder)? {
        let removed = if is_dir {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        removed.map_err(|source| StagingError::Remove { path, source })?;
    }
    Ok(count)
}

/// Returns whether `folder` is a filesystem root, an empty path or the home
/// folder.
///
/// `folder` is compared with symlinks resolved, when it exists.
fn is_unsafe_to_clear(folder: &Path) -> bool {
    let folder = folder.canonicalize().unwrap_or_else(|_| folder.to_owned());
    let home = directories::UserDirs::new().map(|dirs| dirs.home_dir().to_owned());
    folder.parent().is_none() || home.is_some_and(|home| folder == home)
}

// == Grouping music ===========================================================

/// Adds the music files in `folder` and below to `files`.
///
/// # Arguments
///
/// - `root`: the Staging folder, which the added paths are relative to.
/// - `folder`: the folder to walk, `root` or a folder below it.
/// - `files`: where the music files are added, in no particular order.
///
/// # Errors
///
/// [`StagingError::Read`] if `folder` or a folder below it couldn't be
/// listed.
fn collect_music(root: &Path, folder: &Path, files: &mut Vec<PathBuf>) -> Result<(), StagingError> {
    for (path, is_dir) in entries(folder)? {
        if is_dir {
            collect_music(root, &path, files)?;
        } else if is_music(&path) {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

/// Returns the Album folder of `file`, a path relative to the Staging folder.
///
/// That's the folder `file` sits in, or the one above if that's a disc
/// folder. A disc folder directly in the Staging folder is an Album folder
/// of its own, not single tracks.
///
/// # Returns
///
/// The Album folder, relative to the Staging folder, or the empty path when
/// `file` is one of the single tracks.
fn album_folder(file: &Path) -> PathBuf {
    let parent = file.parent().unwrap_or(Path::new(""));
    let album = parent
        .parent()
        .filter(|album| !album.as_os_str().is_empty());
    match (parent.file_name().and_then(|name| name.to_str()), album) {
        (Some(name), Some(album)) if is_disc_folder(name) => album,
        _ => parent,
    }
    .to_path_buf()
}

/// Returns whether `name` is a disc folder's name, in any case.
///
/// That's `Disc 1` as streamrip names them, and `CD 2` or `disc2` from
/// other tools.
fn is_disc_folder(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let Some(number) = name
        .strip_prefix("disc")
        .or_else(|| name.strip_prefix("cd"))
    else {
        return false;
    };
    let number = number.trim_start();
    !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
}

// == Helpers ==================================================================

/// Lists the entries in `folder`, in the order the OS gives them.
///
/// # Returns
///
/// `(path, is_dir)` for each entry, or nothing if `folder` doesn't exist.
/// `is_dir` is false for a symlink, even one to a folder.
///
/// # Errors
///
/// [`StagingError::Read`] if `folder` exists but couldn't be listed, or an
/// entry's type couldn't be read.
fn entries(folder: &Path) -> Result<Vec<(PathBuf, bool)>, StagingError> {
    let read_err = |path: &Path| {
        let path = path.to_owned();
        move |source| StagingError::Read { path, source }
    };
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(read_err(folder)(err)),
    };
    entries
        .map(|entry| {
            let entry = entry.map_err(read_err(folder))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(read_err(&path))?;
            Ok((path, file_type.is_dir()))
        })
        .collect()
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns a temporary Staging folder with `album/01.flac`,
    /// `album/02.flac`, `album/cover/front.jpg`, `loose.mp3` and an empty
    /// folder.
    ///
    /// # Panics
    ///
    /// Panics if the folder or its files can't be created.
    fn staging() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("album/cover")).unwrap();
        fs::create_dir(root.join("empty")).unwrap();
        for file in [
            "album/01.flac",
            "album/02.flac",
            "album/cover/front.jpg",
            "loose.mp3",
        ] {
            fs::write(root.join(file), "x").unwrap();
        }
        dir
    }

    /// Returns the [`MusicGroup`] of `files` in `folder`, `None` for the
    /// single tracks.
    fn group(folder: Option<&str>, files: &[&str]) -> MusicGroup {
        MusicGroup {
            folder: folder.map(PathBuf::from),
            files: files.iter().map(PathBuf::from).collect(),
        }
    }

    // == Music files ==========================================================

    /// Music files are told apart by their extension, in any case.
    #[test]
    fn music_is_told_apart_by_extension_in_any_case() {
        for music in ["01.flac", "a/b/02. Song.MP3", "x.m4a", "x.Opus", "x.dsf"] {
            assert!(is_music(Path::new(music)), "{music}");
        }
        for other in [
            "cover.jpg",
            "folder.png",
            "01.lrc",
            "album.cue",
            "flac",
            "notes",
        ] {
            assert!(!is_music(Path::new(other)), "{other}");
        }
    }

    // == Staging contents =====================================================

    /// Music is grouped by Album folder, with the single tracks last.
    #[test]
    fn music_is_grouped_by_album_folder_with_loose_files_last() {
        let dir = staging();
        assert_eq!(
            music_groups(dir.path()).unwrap(),
            [
                group(Some("album"), &["album/01.flac", "album/02.flac"]),
                group(None, &["loose.mp3"]),
            ]
        );
    }

    /// Disc folders belong to the Album folder above them, however deep.
    #[test]
    fn disc_folders_belong_to_their_album_however_deep_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let files = [
            "ACDC/Back in Black (1980)/01. Hells Bells.flac",
            "ACDC/Live/Disc 1/01. Intro.flac",
            "ACDC/Live/disc2/01. Encore.flac",
            "ACDC/Live/CD 3/01. Outro.flac",
            "ACDC/Live/cover.jpg",
            "Discovery/01. One More Time.flac",
        ];
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "x").unwrap();
        }

        assert_eq!(
            music_groups(dir.path()).unwrap(),
            [
                group(
                    Some("ACDC/Back in Black (1980)"),
                    &["ACDC/Back in Black (1980)/01. Hells Bells.flac"]
                ),
                group(
                    Some("ACDC/Live"),
                    &[
                        "ACDC/Live/CD 3/01. Outro.flac",
                        "ACDC/Live/Disc 1/01. Intro.flac",
                        "ACDC/Live/disc2/01. Encore.flac",
                    ]
                ),
                group(Some("Discovery"), &["Discovery/01. One More Time.flac"]),
            ]
        );
    }

    /// A disc folder directly in the Staging folder is its own group.
    #[test]
    fn a_disc_folder_with_no_album_above_it_is_its_own_group() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("Disc 1")).unwrap();
        fs::write(dir.path().join("Disc 1/01.flac"), "x").unwrap();

        assert_eq!(
            music_groups(dir.path()).unwrap(),
            [group(Some("Disc 1"), &["Disc 1/01.flac"])]
        );
    }

    /// Files in subfolders are counted; folders aren't.
    #[test]
    fn count_includes_files_in_subfolders() {
        let dir = staging();
        assert_eq!(count_files(dir.path()).unwrap(), 4);
    }

    /// A missing folder has no files.
    #[test]
    fn missing_folder_has_no_files() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(count_files(&dir.path().join("not-yet")).unwrap(), 0);
    }

    /// A symlink counts as one file and isn't followed.
    #[cfg(unix)]
    #[test]
    fn symlinks_are_counted_not_followed() {
        let dir = staging();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("a"), "x").unwrap();
        fs::write(outside.path().join("b"), "x").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();
        assert_eq!(count_files(dir.path()).unwrap(), 5);
    }

    // == Comparisons ==========================================================

    /// A Comparison counts what rsync would send and what came since.
    #[test]
    fn a_comparison_counts_what_rsync_would_send_and_what_came_since() {
        let dir = staging();
        let contents = StagingContents::read(dir.path()).unwrap();
        let comparison = Comparison::new(dir.path(), &contents, &[PathBuf::from("album/02.flac")]);
        fs::write(dir.path().join("album/03.flac"), "x").unwrap();
        let now = StagingContents::read(dir.path()).unwrap();

        assert_eq!(now.music_count(), 4);
        assert_eq!(now.other_count(), 1, "the cover");
        assert_eq!(comparison.to_upload(now.music_files()), 2, "02 and 03");
        assert!(comparison.is_uploaded(Path::new("album/01.flac")));
        assert!(comparison.is_uploaded(Path::new("loose.mp3")));
        assert!(comparison.is_for(dir.path()));
        assert!(!comparison.is_for(Path::new("/elsewhere")));
    }

    // == Clearing =============================================================

    /// Clearing empties the folder but keeps it.
    #[test]
    fn clear_empties_the_folder_but_keeps_it() {
        let dir = staging();
        assert_eq!(clear(dir.path()).unwrap(), 4);
        assert!(dir.path().is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// Clearing a missing folder does nothing and doesn't create it.
    #[test]
    fn clearing_a_missing_folder_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-yet");
        assert_eq!(clear(&missing).unwrap(), 0);
        assert!(!missing.exists());
    }

    /// Clearing removes a symlink but not what it points to.
    #[cfg(unix)]
    #[test]
    fn clear_removes_a_symlink_but_not_what_it_points_to() {
        let dir = staging();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("keep"), "x").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).unwrap();

        clear(dir.path()).unwrap();
        assert!(outside.path().join("keep").exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    /// Clearing refuses a filesystem root, an empty path and home.
    #[test]
    fn clear_refuses_the_root_and_the_home_folder() {
        for folder in [Path::new("/"), Path::new("")] {
            assert!(
                matches!(clear(folder), Err(StagingError::Refused { .. })),
                "{folder:?}"
            );
        }
        if let Some(dirs) = directories::UserDirs::new() {
            assert!(matches!(
                clear(dirs.home_dir()),
                Err(StagingError::Refused { .. })
            ));
        }
    }
}
