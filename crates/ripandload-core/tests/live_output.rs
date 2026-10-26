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
