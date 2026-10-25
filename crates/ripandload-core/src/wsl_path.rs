//! Translating Windows paths to the paths WSL sees them at (ADR 0001).

use std::path::Path;

/// `C:\Users\a b\Music` → `/mnt/c/Users/a b/Music`.
///
/// Only absolute paths on a drive letter can be translated; anything else
/// (`/home/a`, `relative\path`, `\\server\share`, `C:no-slash`) gives `None`.
/// Works on the path's text, so it behaves the same on Linux and Windows.
pub fn to_wsl_path(path: &Path) -> Option<String> {
    let path_str = path.to_str()?;
    let mut chars = path_str.chars();

    let drive = chars.next()?;
    if !drive.is_ascii_alphabetic() {
        return None;
    }

    let rest = chars.as_str().strip_prefix(':')?;
    if !rest.starts_with(['\\', '/']) {
        return None;
    }

    Some(format!(
        "/mnt/{}{}",
        drive.to_ascii_lowercase(),
        rest.replace('\\', "/")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wsl(path: &str) -> Option<String> {
        to_wsl_path(Path::new(path))
    }

    #[test]
    fn drive_path_moves_under_mnt() {
        assert_eq!(
            wsl(r"C:\Users\a b\Music").as_deref(),
            Some("/mnt/c/Users/a b/Music")
        );
    }

    #[test]
    fn drive_letter_is_lowercased() {
        assert_eq!(wsl(r"d:\Music").as_deref(), Some("/mnt/d/Music"));
        assert_eq!(wsl(r"E:\Music").as_deref(), Some("/mnt/e/Music"));
    }

    #[test]
    fn forward_slashes_work_too() {
        assert_eq!(
            wsl("C:/Users/a/Music").as_deref(),
            Some("/mnt/c/Users/a/Music")
        );
    }

    #[test]
    fn trailing_slash_is_kept() {
        // rsync cares: `staging/` means "the folder's contents".
        assert_eq!(wsl(r"C:\Music\").as_deref(), Some("/mnt/c/Music/"));
        assert_eq!(wsl(r"C:\").as_deref(), Some("/mnt/c/"));
    }

    #[test]
    fn non_drive_paths_are_none() {
        assert_eq!(wsl("/home/a/Music"), None);
        assert_eq!(wsl(r"Music\ripandload"), None);
        assert_eq!(wsl(r"\\server\share\Music"), None);
        assert_eq!(wsl("C:Music"), None);
        assert_eq!(wsl("1:\\Music"), None);
        assert_eq!(wsl(""), None);
    }
}
