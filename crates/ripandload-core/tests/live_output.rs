//! Checks that [`LiveOutput`] splits recorded streamrip output into log
//! lines and live rows.
//!
//! The fixtures are real recordings of streamrip drawing progress bars. The
//! tests that read a whole recording feed it in small chunks that cut
//! through escape sequences and characters.
//!
//! # Fixtures
//!
//! - `rich13-progress.out`: what streamrip 2.2.0's `ProgressManager` writes
//!   under rich 13.9.4 with `FORCE_COLOR=1 COLUMNS=60`. A log line, two
//!   tracks' progress bars redrawn four times with an error logged halfway,
//!   then another log line.
//! - `rich13-one-row.out`: recorded the same way, with `RichHandler`'s time
//!   and path columns off. One track finishes, so only the "Downloading" row
//!   is left, and an error is logged while the display is that one row high.

#![allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]

use ripandload_core::live_output::LiveOutput;

// == Fixtures =================================================================

/// Two tracks' progress bars, with lines logged before, during and after.
const RECORDED: &[u8] = include_bytes!("fixtures/rich13-progress.out");

/// A Live display that shrinks to one row, with a line logged while it is.
///
/// A one-row redraw doesn't move the cursor up; it only goes back to the
/// row's start.
const ONE_ROW: &[u8] = include_bytes!("fixtures/rich13-one-row.out");

// == Recorded progress ========================================================

/// Log lines come out, in order, and no progress bar does.
#[test]
fn log_lines_come_out_and_progress_does_not() {
    let mut output = LiveOutput::default();
    let mut lines = Vec::new();
    // Chunks cut through escape sequences and multi-byte characters.
    for chunk in RECORDED.chunks(7) {
        lines.extend(output.feed(chunk));
    }
    lines.extend(output.finish());

    assert_eq!(
        lines,
        [
            "[13:02:04] INFO     Logging in to qobuz             rec.py:7",
            "           ERROR    Track 03 not available for     rec.py:15",
            "                    stream on qobuz",
            "[13:02:05] INFO     Finished                       rec.py:19",
        ]
    );
    assert!(output.live_rows().is_empty());
}

/// The live rows show every row of the latest frame, its last one included.
#[test]
fn live_rows_show_the_whole_latest_frame() {
    // Up to the redraw that follows the 50% frame. rich ends a frame without
    // a newline, so its last row must show before any newline comes.
    let second_redraw = RECORDED
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\r')
        .nth(1)
        .unwrap()
        .0;
    let mut output = LiveOutput::default();
    output.feed(&RECORDED[..second_redraw]);

    let rows = output.live_rows();
    assert_eq!(rows.len(), 3, "{rows:#?}");
    assert!(rows[0].contains("Downloading Discovery"), "{rows:#?}");
    assert!(rows[1].starts_with("01. One More Time"), "{rows:#?}");
    assert!(rows[1].contains("50.0%"), "{rows:#?}");
    assert!(rows[2].starts_with("02. Aerodynamic"), "{rows:#?}");
    assert!(rows[2].contains("50.0%"), "{rows:#?}");
}

/// A Download cut off mid-display leaves no progress in the finished lines.
#[test]
fn a_cancelled_download_leaves_no_progress_behind() {
    let last_redraw = RECORDED.iter().rposition(|byte| *byte == b'\r').unwrap();
    let mut output = LiveOutput::default();
    output.feed(&RECORDED[..last_redraw]);

    assert!(!output.finish().iter().any(|line| line.contains('%')));
    assert!(output.live_rows().is_empty());
}

/// A line logged while the Live display is one row high still comes out.
#[test]
fn a_line_logged_while_the_display_is_one_row_reaches_the_log() {
    let mut output = LiveOutput::default();
    let mut lines = Vec::new();
    for chunk in ONE_ROW.chunks(5) {
        lines.extend(output.feed(chunk));
    }
    lines.extend(output.finish());

    assert_eq!(
        lines,
        [
            "ERROR    Track 02 not available for stream on qobuz",
            "INFO     Finished",
        ]
    );
}

// == Broken output ============================================================

/// The text after an escape sequence that never ends still comes out.
#[test]
fn an_escape_sequence_that_never_ends_doesnt_swallow_the_rest() {
    let mut input = b"\x1b]8;".to_vec();
    input.extend(std::iter::repeat_n(b'x', 10_000));
    input.extend(b"\nhello\n");

    let mut output = LiveOutput::default();
    let mut lines = Vec::new();
    for chunk in input.chunks(1000) {
        lines.extend(output.feed(chunk));
    }

    assert_eq!(lines.last().map(String::as_str), Some("hello"));
}

/// Cursor up above the top of the Live display keeps the rows it moved over.
#[test]
fn cursor_up_above_the_live_display_doesnt_panic() {
    let mut output = LiveOutput::default();

    let finished = output.feed(b"a\nb\n\x1b[?25l\x1b[2A");

    assert!(finished.is_empty());
    assert_eq!(output.finish(), ["a", "b"]);
}

/// With the Live display off, cursor up past the returned rows stays on the
/// first row kept.
#[test]
fn cursor_up_past_returned_rows_stops_at_the_first_kept_row() {
    let mut output = LiveOutput::default();

    assert_eq!(output.feed(b"a\nb\n"), ["a", "b"]);

    assert_eq!(output.feed(b"\x1b[3Axyz\n"), ["xyz"]);
}
