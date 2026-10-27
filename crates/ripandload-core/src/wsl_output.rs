//! Tool output decoded to text, including what comes through `wsl.exe`.
//!
//! The tools inside WSL print UTF-8, but `wsl.exe`'s own messages ("There is
//! no distribution with the supplied name.") are UTF-16LE on WSL versions
//! that ignore `WSL_UTF8`. Both can arrive on the same stream, so each line
//! is decoded on its own (ADR 0001, in `docs/adr/`).
//!
//! This module only decodes bytes. Splitting output into lines is [`Job`]'s
//! job, and replaying progress bars is [`LiveOutput`]'s.
//!
//! [`Job`]: crate::job::Job
//! [`LiveOutput`]: crate::live_output::LiveOutput

// == Decoding =================================================================

/// Returns a tool's output as text, however the platform encodes it.
///
/// On Windows, where the output may come from `wsl.exe`, it goes through
/// [`decode_wsl_output`]. Everywhere else it is UTF-8, and anything that
/// isn't becomes `�`.
///
/// # Arguments
///
/// - `bytes`: one line of output, as [`Job`] splits it on `b'\n'`, or a
///   whole output.
///
/// [`Job`]: crate::job::Job
pub fn decode_tool_output(bytes: &[u8]) -> String {
    if cfg!(windows) {
        decode_wsl_output(bytes)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Returns output that came through `wsl.exe` as text, from UTF-8 or UTF-16LE.
///
/// It is UTF-16LE if it holds a NUL byte, which text never does, and UTF-8
/// otherwise. Either way, anything that can't be decoded becomes `�`.
///
/// # Arguments
///
/// - `bytes`: one line of output, as [`Job`] splits it on `b'\n'`, or a
///   whole output. A UTF-16LE line after the first starts with the NUL that
///   was the second half of the previous line's `\n`; that NUL is dropped.
///
/// [`Job`]: crate::job::Job
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    let bytes = if bytes.first() == Some(&0) && bytes.len() % 2 == 1 {
        &bytes[1..]
    } else {
        bytes
    };

    if bytes.contains(&0) {
        // UTF-16LE. A lone byte left at the end becomes `�`.
        let char_list = bytes
            .chunks(2)
            .map(|pair| match pair {
                [low, high] => u16::from_le_bytes([*low, *high]),
                _ => char::REPLACEMENT_CHARACTER as u16,
            })
            .collect::<Vec<u16>>();
        String::from_utf16_lossy(&char_list)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns `text` encoded as UTF-16LE, the way `wsl.exe` writes it.
    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    // == UTF-8 ================================================================

    /// UTF-8, accented letters included, comes out as it went in.
    #[test]
    fn utf8_is_kept_as_it_is() {
        assert_eq!(
            decode_wsl_output(b"rip, version 2.2.0"),
            "rip, version 2.2.0"
        );
        assert_eq!(decode_wsl_output("Città – Così".as_bytes()), "Città – Così");
    }

    /// Bytes that aren't valid UTF-8 become `�`.
    #[test]
    fn broken_utf8_becomes_replacement_characters() {
        assert_eq!(decode_wsl_output(b"caf\xe9 ok"), "caf\u{FFFD} ok");
    }

    /// No bytes give an empty string.
    #[test]
    fn empty_is_empty() {
        assert_eq!(decode_wsl_output(b""), "");
    }

    // == UTF-16LE =============================================================

    /// A UTF-16LE message from `wsl.exe` is decoded.
    #[test]
    fn utf16le_is_decoded() {
        let bytes = utf16le("There is no distribution with the supplied name.");
        assert_eq!(
            decode_wsl_output(&bytes),
            "There is no distribution with the supplied name."
        );
    }

    /// UTF-16LE beyond ASCII, emoji included, is decoded.
    #[test]
    fn utf16le_beyond_ascii_is_decoded() {
        let bytes = utf16le("Non è stata trovata alcuna distribuzione 🎵");
        assert_eq!(
            decode_wsl_output(&bytes),
            "Non è stata trovata alcuna distribuzione 🎵"
        );
    }

    /// A UTF-16LE line after the first drops the NUL the split left behind.
    #[test]
    fn later_utf16le_lines_drop_the_nul_left_by_the_split() {
        // Splitting on b'\n' leaves the `\n`'s second byte, a NUL, at the
        // start of the second line.
        let output = utf16le("first\nError code: Wsl/Service/WSL_E_DISTRO_NOT_FOUND");
        let lines: Vec<&[u8]> = output.split(|byte| *byte == b'\n').collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(decode_wsl_output(lines[0]), "first");
        assert_eq!(
            decode_wsl_output(lines[1]),
            "Error code: Wsl/Service/WSL_E_DISTRO_NOT_FOUND"
        );
    }

    /// A whole UTF-16LE output keeps its `\r\n` line breaks.
    #[test]
    fn whole_utf16le_output_keeps_its_line_breaks() {
        let bytes = utf16le("one\r\ntwo\r\n");
        assert_eq!(decode_wsl_output(&bytes), "one\r\ntwo\r\n");
    }
}
