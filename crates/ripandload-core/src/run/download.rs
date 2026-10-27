//! A Download: one `rip` run, read as it comes so its progress bars can be
//! shown.
//!
//! rip's stdout and stderr are each replayed on their own [`LiveOutput`],
//! so neither lands in the middle of a line of the other. rich draws on
//! stdout; stderr is only Python's own complaints. Lines that scroll above
//! the Live display go to the log; the Live display itself is only shown,
//! through `live_rows`.

use crate::command::{self, CommandSpec, Tool};
use crate::live_output::LiveOutput;
use crate::process::Stream;

use super::check_tools::rip_missing_hint;
use super::{Failure, Kind, Outcome};

// == Download =================================================================

/// A Download.
#[derive(Debug, Default)]
pub(crate) struct Download {
    /// rip's stdout, with the progress bars.
    stdout: LiveOutput,
    /// rip's stderr.
    stderr: LiveOutput,
}

impl Download {
    /// A Download with no output yet, and its one command.
    ///
    /// # Arguments
    ///
    /// - `config`: the Streamrip config's path, as `rip` sees it.
    /// - `urls`: the URLs to download, in this order.
    /// - `quality`: the Quality for every Source; `None` keeps the one in
    ///   the Streamrip config.
    ///
    /// # Returns
    ///
    /// The kind, and the `rip` run from [`command::download_spec`].
    pub(crate) fn new(
        config: &str,
        urls: &[&str],
        quality: Option<u8>,
    ) -> (Self, Vec<CommandSpec>) {
        let spec = command::download_spec(config, urls, quality);
        return (Self::default(), vec![spec]);
    }

    /// Returns the output that `stream`'s bytes feed.
    fn for_stream(&mut self, stream: Stream) -> &mut LiveOutput {
        match stream {
            Stream::Stdout => return &mut self.stdout,
            Stream::Stderr => return &mut self.stderr,
        }
    }
}

/// Reads `rip`'s output raw, for its Live display, and always ends with
/// [`Outcome::Downloaded`].
impl Kind for Download {
    fn name(&self) -> &'static str {
        return "Download";
    }

    fn says_finished(&self) -> bool {
        return true;
    }

    // rip's progress bars redraw themselves, so it's read as it comes.
    fn reads_raw(&self, spec: &CommandSpec) -> bool {
        return spec.program == Tool::Rip;
    }

    fn on_output(&mut self, stream: Stream, bytes: &[u8]) -> Vec<String> {
        return self.for_stream(stream).feed(bytes);
    }

    fn finish_output(&mut self) -> Vec<String> {
        let mut lines = self.stdout.finish();
        lines.extend(self.stderr.finish());
        return lines;
    }

    fn live_rows(&self) -> Vec<String> {
        return self.stdout.live_rows();
    }

    fn on_failure(&mut self, spec: &CommandSpec, code: i32, wsl: bool) -> Failure {
        return Failure {
            hint: rip_missing_hint(spec, code, wsl),
            ..Failure::default()
        };
    }

    fn outcome(&mut self, _succeeded: bool) -> Outcome {
        return Outcome::Downloaded;
    }
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;
    use crate::run::{Next, SHELL_NOT_FOUND};

    // == Fixtures =============================================================

    /// rip's stdout for two tracks, progress bars and all, as recorded for
    /// `tests/live_output.rs`.
    const RECORDED: &[u8] = include_bytes!("../../tests/fixtures/rich13-progress.out");

    /// Returns a Download of one track and its `rip` run.
    fn download() -> (Download, CommandSpec) {
        let (kind, mut specs) = Download::new("/c/rip.toml", &["https://x/1"], None);
        return (kind, specs.remove(0));
    }

    // == Output ===============================================================

    /// `rip` is read raw.
    #[test]
    fn rip_is_read_raw() {
        let (kind, spec) = download();

        assert!(kind.reads_raw(&spec));
        assert!(!kind.reads_raw(&command::wsl_check_spec()));
    }

    /// Raw bytes give finished lines on either stream.
    #[test]
    fn raw_bytes_give_lines() {
        let (mut kind, _) = download();

        let out = kind.on_output(Stream::Stdout, b"Logging in\nSearching");
        let err = kind.on_output(Stream::Stderr, b"a warning\n");

        assert_eq!(out, ["Logging in"]);
        assert_eq!(err, ["a warning"]);
    }

    /// stdout's Live display shows in the live rows until the output is
    /// finished.
    #[test]
    fn stdout_gives_live_rows_until_finished() {
        let (mut kind, _) = download();
        // Up to the redraw that follows the first frame, so the Live display
        // is on.
        let second_redraw = RECORDED
            .iter()
            .enumerate()
            .filter(|(_, byte)| **byte == b'\r')
            .nth(1)
            .unwrap()
            .0;

        kind.on_output(Stream::Stdout, &RECORDED[..second_redraw]);
        assert!(!kind.live_rows().is_empty());

        kind.finish_output();
        assert!(kind.live_rows().is_empty());
    }

    /// What's left on either stream comes out when the output finishes,
    /// stdout first.
    #[test]
    fn finish_hands_over_the_rest() {
        let (mut kind, _) = download();
        kind.on_output(Stream::Stderr, b"err");
        kind.on_output(Stream::Stdout, b"out");

        assert_eq!(kind.finish_output(), ["out", "err"]);
    }

    // == Ending ===============================================================

    /// `rip` missing inside WSL says where to put it, without the probe.
    #[test]
    fn rip_missing_in_wsl_has_a_hint() {
        let (mut kind, spec) = download();

        let failure = kind.on_failure(&spec, SHELL_NOT_FOUND, true);

        assert!(failure.hint.unwrap().starts_with("streamrip isn't where"));
        assert_eq!(failure.next, Next::CarryOn);
        assert_eq!(kind.on_failure(&spec, 1, true), Failure::default());
    }

    /// A Download ends with `Downloaded`, however it went.
    #[test]
    fn always_downloaded() {
        let (mut kind, _) = download();

        assert!(matches!(kind.outcome(false), Outcome::Downloaded));
    }
}
