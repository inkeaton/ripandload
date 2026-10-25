//! What to run, as plain values. Turning these into real processes is
//! `tool_env`'s job, so the same `CommandSpec` works natively and under WSL.

use std::fmt;

/// The streamrip version the app's Streamrip config is written for.
pub const STREAMRIP_VERSION: &str = "2.2.0";

/// A program the app runs. `tool_env` decides where each one lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Rip,
    Rsync,
}

impl Tool {
    /// The executable's name, as looked up on PATH.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Rip => "rip",
            Tool::Rsync => "rsync",
        }
    }
}

/// One command line: which tool, its arguments and extra environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: Tool,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    fn new(program: Tool, args: &[&str]) -> Self {
        Self {
            program,
            args: args.iter().map(|arg| arg.to_string()).collect(),
            env: Vec::new(),
        }
    }
}

/// Shows the command the way it would be typed, for the log.
impl fmt::Display for CommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.program.name())?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

/// The commands behind "Check tools": `rip --version`, then `rsync --version`.
pub fn version_specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new(Tool::Rip, &["--version"]),
        CommandSpec::new(Tool::Rsync, &["--version"]),
    ]
}

/// `rip --config-path <config> config path`. If `config` doesn't exist,
/// streamrip creates a default one there before printing its path.
pub fn seed_config_spec(config: &str) -> CommandSpec {
    CommandSpec::new(Tool::Rip, &["--config-path", config, "config", "path"])
}

/// A Download: `rip --config-path <config> [-q N] --no-progress url <urls>…`.
/// `quality` overrides the Settings' Quality for every Source in this run.
/// The Streamrip config must already be rendered.
pub fn download_spec(config: &str, urls: &[&str], quality: Option<u8>) -> CommandSpec {
    let mut spec = CommandSpec::new(Tool::Rip, &["--config-path", config]);
    if let Some(quality) = quality {
        spec.args.extend(["-q".to_owned(), quality.to_string()]);
    }
    spec.args
        .extend(["--no-progress", "url"].map(str::to_owned));
    spec.args.extend(urls.iter().map(|url| url.to_string()));
    spec
}

/// Finds the version in `rip --version` output, e.g. `rip, version 2.2.0`.
pub fn parse_rip_version(line: &str) -> Option<&str> {
    let (_, version) = line.trim().split_once(", version ")?;
    Some(version.trim()).filter(|version| !version.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_specs_check_rip_then_rsync() {
        let specs = version_specs();
        let lines: Vec<String> = specs.iter().map(ToString::to_string).collect();
        assert_eq!(lines, ["rip --version", "rsync --version"]);
        assert!(specs.iter().all(|spec| spec.env.is_empty()));
    }

    #[test]
    fn seed_config_spec_points_rip_at_the_streamrip_config() {
        let spec = seed_config_spec("/home/a b/streamrip.toml");
        assert_eq!(spec.program, Tool::Rip);
        assert_eq!(
            spec.args,
            [
                "--config-path",
                "/home/a b/streamrip.toml",
                "config",
                "path"
            ]
        );
    }

    #[test]
    fn download_spec_uses_the_settings_quality_by_default() {
        let spec = download_spec(
            "/cfg/streamrip.toml",
            &["https://open.qobuz.com/track/1"],
            None,
        );
        assert_eq!(
            spec.to_string(),
            "rip --config-path /cfg/streamrip.toml --no-progress url https://open.qobuz.com/track/1"
        );
        assert!(spec.env.is_empty());
    }

    #[test]
    fn download_spec_passes_a_quality_override() {
        let spec = download_spec(
            "/cfg/streamrip.toml",
            &["https://deezer.com/track/2"],
            Some(1),
        );
        assert_eq!(
            spec.args,
            [
                "--config-path",
                "/cfg/streamrip.toml",
                "-q",
                "1",
                "--no-progress",
                "url",
                "https://deezer.com/track/2",
            ]
        );
    }

    #[test]
    fn download_spec_takes_every_url_in_order() {
        let spec = download_spec("/a b/streamrip.toml", &["u1", "u2", "u3"], None);
        assert_eq!(spec.program, Tool::Rip);
        assert_eq!(spec.args[1], "/a b/streamrip.toml");
        assert_eq!(spec.args[spec.args.len() - 4..], ["url", "u1", "u2", "u3"]);
    }

    #[test]
    fn rip_version_is_parsed() {
        assert_eq!(parse_rip_version("rip, version 2.2.0\n"), Some("2.2.0"));
        assert_eq!(parse_rip_version("rip, version 2.1.0"), Some("2.1.0"));
    }

    #[test]
    fn other_lines_have_no_rip_version() {
        assert_eq!(parse_rip_version("Usage: rip [OPTIONS]"), None);
        assert_eq!(parse_rip_version("rip, version "), None);
        assert_eq!(parse_rip_version(""), None);
    }
}
