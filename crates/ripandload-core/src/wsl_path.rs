//! Windows paths translated to the paths the tools inside WSL see.
//!
//! On Windows the tools run inside a WSL distro (ADR 0001, in `docs/adr/`),
//! so every path the app hands them must be translated first. Two kinds of
//! path can be: paths on a drive, which WSL mounts under `/mnt/<drive>/`,
//! and the distros' own files, which Windows shows on its WSL share as
//! `\\wsl.localhost\<distro>\…`.
//!
//! Both translations work on the path's text alone, so they behave the same
//! on Linux and Windows and never touch the disk. Choosing between them, and
//! checking that a share path is in the tools' own distro, is
//! [`ToolEnv::tool_path`]'s job.
//!
//! [`ToolEnv::tool_path`]: crate::tool_env::ToolEnv::tool_path

use std::path::Path;

// == Drive paths ==============================================================

/// Returns the path WSL mounts a Windows drive path at.
///
/// `C:\Users\a b\Music` becomes `/mnt/c/Users/a b/Music`: the drive letter
/// is lowercased, backslashes become slashes, and a trailing slash is kept.
///
/// # Arguments
///
/// - `path`: a Windows path, with either kind of slash.
///
/// # Returns
///
/// The path under `/mnt/<drive>/`, or `None` when `path` isn't valid Unicode
/// or isn't an absolute path on a drive letter: `/home/a`, `relative\path`,
/// `\\server\share` and `C:no-slash` all give `None`.
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

// == WSL shares ===============================================================

/// Returns the distro a path on Windows' WSL share is in, and the path in it.
///
/// `\\wsl.localhost\Ubuntu\home\a` gives `("Ubuntu", "/home/a")`. Older
/// Windows uses `\\wsl$\` instead; both are accepted, in any case and with
/// either kind of slash.
///
/// # Arguments
///
/// - `path`: a path as Windows sees it.
///
/// # Returns
///
/// `(distro, path inside the distro)`, where the distro's own root is `/`.
/// `None` when `path` isn't valid Unicode, isn't on the WSL share, or names
/// no distro.
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

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns what [`to_wsl_path`] gives for `path`.
    fn wsl(path: &str) -> Option<String> {
        to_wsl_path(Path::new(path))
    }

    /// Returns what [`from_wsl_share`] gives for `path`.
    fn share(path: &str) -> Option<(String, String)> {
        from_wsl_share(Path::new(path))
    }

    /// Returns what [`from_wsl_share`] should give for `path` in `distro`.
    fn found(distro: &str, path: &str) -> Option<(String, String)> {
        Some((distro.to_owned(), path.to_owned()))
    }

    // == Drive paths ==========================================================

    /// A drive path moves under `/mnt/<drive>`, spaces and all.
    #[test]
    fn drive_path_moves_under_mnt() {
        assert_eq!(
            wsl(r"C:\Users\a b\Music").as_deref(),
            Some("/mnt/c/Users/a b/Music")
        );
    }

    /// The drive letter is lowercased, whatever its case.
    #[test]
    fn drive_letter_is_lowercased() {
        assert_eq!(wsl(r"d:\Music").as_deref(), Some("/mnt/d/Music"));
        assert_eq!(wsl(r"E:\Music").as_deref(), Some("/mnt/e/Music"));
    }

    /// A drive path written with forward slashes is translated too.
    #[test]
    fn forward_slashes_work_too() {
        assert_eq!(
            wsl("C:/Users/a/Music").as_deref(),
            Some("/mnt/c/Users/a/Music")
        );
    }

    /// A trailing slash is kept: rsync reads `staging/` as its contents.
    #[test]
    fn trailing_slash_is_kept() {
        assert_eq!(wsl(r"C:\Music\").as_deref(), Some("/mnt/c/Music/"));
        assert_eq!(wsl(r"C:\").as_deref(), Some("/mnt/c/"));
    }

    /// A path that isn't absolute on a drive letter gives `None`.
    #[test]
    fn non_drive_paths_are_none() {
        assert_eq!(wsl("/home/a/Music"), None);
        assert_eq!(wsl(r"Music\ripandload"), None);
        assert_eq!(wsl(r"\\server\share\Music"), None);
        assert_eq!(wsl("C:Music"), None);
        assert_eq!(wsl("1:\\Music"), None);
        assert_eq!(wsl(""), None);
    }

    // == WSL shares ===========================================================

    /// A `\\wsl.localhost` path gives its distro and the path inside it.
    #[test]
    fn wsl_localhost_share_gives_the_distro_and_its_own_path() {
        assert_eq!(
            share(r"\\wsl.localhost\Ubuntu\home\user\Music"),
            found("Ubuntu", "/home/user/Music")
        );
    }

    /// A path on older Windows' `\\wsl$` share works too.
    #[test]
    fn older_wsl_dollar_share_works_too() {
        assert_eq!(
            share(r"\\wsl$\Debian\home\a b\.ssh\key"),
            found("Debian", "/home/a b/.ssh/key")
        );
    }

    /// The share's host is case-insensitive, and its slashes can be forward.
    #[test]
    fn share_host_is_case_insensitive_and_slashes_can_be_forward() {
        assert_eq!(
            share("//WSL.LOCALHOST/Ubuntu/home/user/"),
            found("Ubuntu", "/home/user/")
        );
    }

    /// A distro's root, with or without a trailing slash, is `/`.
    #[test]
    fn distro_root_is_slash() {
        assert_eq!(share(r"\\wsl.localhost\Ubuntu"), found("Ubuntu", "/"));
        assert_eq!(share(r"\\wsl.localhost\Ubuntu\"), found("Ubuntu", "/"));
    }

    /// Other shares, a share with no distro and local paths give `None`.
    #[test]
    fn other_paths_are_not_wsl_shares() {
        assert_eq!(share(r"\\nas\music"), None);
        assert_eq!(share(r"\\wsl.localhost"), None);
        assert_eq!(share(r"\\wsl.localhost\"), None);
        assert_eq!(share(r"C:\Users\a"), None);
        assert_eq!(share("/home/a"), None);
        assert_eq!(share(""), None);
    }
}
