//! Reading what comes back through `wsl.exe` (ADR 0001). The tools inside WSL
//! print UTF-8, but `wsl.exe`'s own messages ("There is no distribution with
//! the supplied name.") are UTF-16LE on versions that ignore `WSL_UTF8`.

/// Output from a tool, as text: through `decode_wsl_output` on Windows,
/// where it may come from `wsl.exe`, and as UTF-8 everywhere else.
pub fn decode_tool_output(bytes: &[u8]) -> String {
    if cfg!(windows) {
        decode_wsl_output(bytes)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// One line of output as `Job` splits it on `b'\n'`, or a whole output.
///
/// It's UTF-16LE if it contains a NUL byte, which text never does. A line
/// after the first then starts with the NUL that was the second half of the
/// previous line's `\n`; that one is dropped. Otherwise it's UTF-8.
/// Either way, anything that can't be decoded becomes `�`.
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    let bytes = if bytes.first() == Some(&0) && bytes.len() % 2 == 1 {
        &bytes[1..]
    } else {
        bytes
    };

    if bytes.contains(&0) {
        // this is UTF-16 coded
        let char_list = bytes
            .chunks(2)
            .map(|pair| match pair {
                [low, high] => u16::from_le_bytes([*low, *high]),
                _ => char::REPLACEMENT_CHARACTER as u16,
            })
            .collect::<Vec<u16>>();
        String::from_utf16_lossy(&char_list)
    } else {
        // this is UTF-8 coded
        String::from_utf8_lossy(bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16le(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn utf8_is_kept_as_it_is() {
        assert_eq!(
            decode_wsl_output(b"rip, version 2.2.0"),
            "rip, version 2.2.0"
        );
        assert_eq!(decode_wsl_output("Città – Così".as_bytes()), "Città – Così");
    }

    #[test]
    fn broken_utf8_becomes_replacement_characters() {
        assert_eq!(decode_wsl_output(b"caf\xe9 ok"), "caf\u{FFFD} ok");
    }

    #[test]
    fn empty_is_empty() {
        assert_eq!(decode_wsl_output(b""), "");
    }

    #[test]
    fn utf16le_is_decoded() {
        let bytes = utf16le("There is no distribution with the supplied name.");
        assert_eq!(
            decode_wsl_output(&bytes),
            "There is no distribution with the supplied name."
        );
    }

    #[test]
    fn utf16le_beyond_ascii_is_decoded() {
        let bytes = utf16le("Non è stata trovata alcuna distribuzione 🎵");
        assert_eq!(
            decode_wsl_output(&bytes),
            "Non è stata trovata alcuna distribuzione 🎵"
        );
    }

    #[test]
    fn later_utf16le_lines_drop_the_nul_left_by_the_split() {
        // "first\nError code: Wsl/…" split on b'\n' leaves a NUL at the start
        // of the second line.
        let output = utf16le("first\nError code: Wsl/Service/WSL_E_DISTRO_NOT_FOUND");
        let lines: Vec<&[u8]> = output.split(|byte| *byte == b'\n').collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(decode_wsl_output(lines[0]), "first");
        assert_eq!(
            decode_wsl_output(lines[1]),
            "Error code: Wsl/Service/WSL_E_DISTRO_NOT_FOUND"
        );
    }

    #[test]
    fn whole_utf16le_output_keeps_its_line_breaks() {
        let bytes = utf16le("one\r\ntwo\r\n");
        assert_eq!(decode_wsl_output(&bytes), "one\r\ntwo\r\n");
    }
}
