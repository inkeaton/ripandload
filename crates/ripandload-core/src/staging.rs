//! The Staging folder: how many files are waiting to be Uploaded, and
//! clearing them out afterwards.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::StagingError;

/// Counts the files in `folder` and every folder below it. Folders themselves
/// don't count, and symlinks are counted, not followed. A missing folder has 0.
pub fn count_files(folder: &Path) -> Result<usize, StagingError> {
    let mut count = 0;
    for (path, is_dir) in entries(folder)? {
        count += if is_dir { count_files(&path)? } else { 1 };
    }
    Ok(count)
}

/// Deletes everything inside `folder`, but never `folder` itself.
/// Returns how many files went. Refuses a filesystem root or the home folder,
/// in case the Staging folder was set to one by mistake.
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

/// The filesystem root, an empty path or the home folder.
fn is_unsafe_to_clear(folder: &Path) -> bool {
    let folder = folder.canonicalize().unwrap_or_else(|_| folder.to_owned());
    let home = directories::UserDirs::new().map(|dirs| dirs.home_dir().to_owned());
    folder.parent().is_none() || home.is_some_and(|home| folder == home)
}

/// `(path, is_dir)` for each entry in `folder`, or nothing if it doesn't exist.
/// `is_dir` is false for a symlink, even one to a folder.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `album/01.flac`, `album/02.flac`, `album/cover/front.jpg`, `loose.mp3`
    /// and an empty folder.
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

    #[test]
    fn count_includes_files_in_subfolders() {
        let dir = staging();
        assert_eq!(count_files(dir.path()).unwrap(), 4);
    }

    #[test]
    fn missing_folder_has_no_files() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(count_files(&dir.path().join("not-yet")).unwrap(), 0);
    }

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

    #[test]
    fn clear_empties_the_folder_but_keeps_it() {
        let dir = staging();
        assert_eq!(clear(dir.path()).unwrap(), 4);
        assert!(dir.path().is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn clearing_a_missing_folder_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("not-yet");
        assert_eq!(clear(&missing).unwrap(), 0);
        assert!(!missing.exists());
    }

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
