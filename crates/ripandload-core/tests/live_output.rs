//! `rich13-progress.out` is what streamrip 2.2.0's `ProgressManager` writes
//! under rich 13.9.4 with `FORCE_COLOR=1 COLUMNS=60`: a log line, two tracks'
//! progress bars redrawn four times with an error logged halfway, then
//! another log line.

use ripandload_core::live_output::LiveOutput;

const RECORDED: &[u8] = include_bytes!("fixtures/rich13-progress.out");

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

#[test]
fn a_cancelled_download_leaves_no_progress_behind() {
    let last_redraw = RECORDED.iter().rposition(|byte| *byte == b'\r').unwrap();
    let mut output = LiveOutput::default();
    output.feed(&RECORDED[..last_redraw]);

    assert!(!output.finish().iter().any(|line| line.contains('%')));
    assert!(output.live_rows().is_empty());
}

/// Recorded like `rich13-progress.out`, with RichHandler's time and path
/// columns off: one track finishes, so only the "Downloading" row is left,
/// and an error is logged while the display is that one row high. A one-row
/// redraw doesn't move the cursor up; it only goes back to the row's start.
const ONE_ROW: &[u8] = include_bytes!("fixtures/rich13-one-row.out");

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
