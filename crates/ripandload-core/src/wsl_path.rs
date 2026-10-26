//! Translating Windows paths to the paths WSL sees them at (ADR 0001): drive
//! paths like `C:\…`, and the distros' own files that Windows shows as
//! `\\wsl.localhost\<distro>\…`.

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

/// `\\wsl.localhost\Ubuntu\home\a` → `("Ubuntu", "/home/a")`: the distro a
/// path from Windows' WSL share is in, and the path inside it.
///
/// Older Windows uses `\\wsl$\` instead; both are accepted, in any case and
/// with either slash. Anything else, or a share with no distro, gives `None`.
pub fn from_wsl_share(path: &Path) -> Option<(String, String)> {
    let text = path.to_str()?.replace('\\', "/");
    let rest = text.strip_prefix("//")?;
    let (host, rest) = rest.split_once('/')?;
    if !host.eq_ignore_ascii_case("wsl.localhost") && !host.eq_ignore_ascii_case("wsl$") {
        return None;
    }
    let (distro, inside) = rest.split_once('/').unwrap_or((rest, ""));
    if distro.is_empty() {
        return None;
    }
    Some((distro.to_owned(), format!("/{inside}")))
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

    fn share(path: &str) -> Option<(String, String)> {
        from_wsl_share(Path::new(path))
    }

    fn found(distro: &str, path: &str) -> Option<(String, String)> {
        Some((distro.to_owned(), path.to_owned()))
    }

    #[test]
    fn wsl_localhost_share_gives_the_distro_and_its_own_path() {
        assert_eq!(
            share(r"\\wsl.localhost\Ubuntu\home\user\Music"),
            found("Ubuntu", "/home/user/Music")
        );
    }

    #[test]
    fn older_wsl_dollar_share_works_too() {
        assert_eq!(
            share(r"\\wsl$\Debian\home\a b\.ssh\key"),
            found("Debian", "/home/a b/.ssh/key")
        );
    }

    #[test]
    fn share_host_is_case_insensitive_and_slashes_can_be_forward() {
        assert_eq!(
            share("//WSL.LOCALHOST/Ubuntu/home/user/"),
            found("Ubuntu", "/home/user/")
        );
    }

    #[test]
    fn distro_root_is_slash() {
        assert_eq!(share(r"\\wsl.localhost\Ubuntu"), found("Ubuntu", "/"));
        assert_eq!(share(r"\\wsl.localhost\Ubuntu\"), found("Ubuntu", "/"));
    }

    #[test]
    fn other_paths_are_not_wsl_shares() {
        assert_eq!(share(r"\\nas\music"), None);
        assert_eq!(share(r"\\wsl.localhost"), None);
        assert_eq!(share(r"\\wsl.localhost\"), None);
        assert_eq!(share(r"C:\Users\a"), None);
        assert_eq!(share("/home/a"), None);
        assert_eq!(share(""), None);
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
