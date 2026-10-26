//! A Download's output, as a terminal would show it.
//!
//! streamrip draws its progress bars with rich's `Live` display: it hides the
//! cursor, draws a few rows, and ten times a second goes back to the start
//! of the row (`\r`), moves the cursor up over the others (`ESC[1A`), clears
//! each (`ESC[2K`) and draws them again. A log message is printed the same
//! way, in place of the bars, with the bars drawn again below it. So a row
//! above where the last redraw started writing will never change again:
//! that's a finished line for the log. The rows from there down are the live
//! rows, and when the cursor shows again (`ESC[?25h`) the display is over and
//! they're dropped, so progress never reaches the log.
//!
//! Only what rich writes is understood: those three sequences, `\r` and `\n`.
//! Colours, hyperlinks and anything else are skipped.

const ESC: u8 = 0x1b;

/// Longer than any escape sequence rich writes; a hyperlink is a few hundred bytes.
const MAX_PENDING: usize = 4096;

#[derive(Debug, Default)]
pub struct LiveOutput {
    /// The rows not yet handed out as finished lines.
    rows: Vec<Vec<char>>,
    /// The cursor, as an index into `rows`.
    row: usize,
    col: usize,
    /// While the Live display is on: the row it starts at.
    live_top: Option<usize>,
    /// A redraw began (`\r`, then the cursor going up over the old rows) and
    /// hasn't written anything yet. Until it does, a chunk may have ended
    /// partway up, so `live_top` stays put.
    redrawing: bool,
    /// The start of an escape sequence or character the last chunk cut off.
    pending: Vec<u8>,
}

/// One step of the output.
enum Token {
    Char(char),
    /// `ESC [ params final`.
    Csi {
        params: String,
        action: char,
    },
    /// Any other escape sequence, or a byte that isn't UTF-8.
    Skip,
}

impl LiveOutput {
    /// Takes the next bytes of output, in chunks of any size, and returns the
    /// lines that are now finished.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut input = std::mem::take(&mut self.pending);
        input.extend_from_slice(bytes);

        let mut at = 0;
        while at < input.len() {
            match next_token(&input[at..]) {
                Some((token, len)) => {
                    at += len;
                    self.apply(token);
                }
                // No escape sequence is this long: this one never ends, so
                // skip its ESC and read on.
                None if input.len() - at > MAX_PENDING => at += 1,
                None => break,
            }
        }
        self.pending = input[at..].to_vec();
        self.take_finished()
    }

    /// After the process has exited: everything left over, except a Live
    /// display that never ended, e.g. after Cancel.
    pub fn finish(&mut self) -> Vec<String> {
        if let Some(top) = self.live_top.take() {
            self.rows.truncate(top);
        }
        let mut lines: Vec<String> = self.rows.drain(..).map(|row| text(&row)).collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        *self = Self::default();
        lines
    }

    /// What the Live display shows right now; empty when it's off.
    pub fn live_rows(&self) -> Vec<String> {
        let Some(top) = self.live_top else {
            return Vec::new();
        };
        let mut rows: Vec<String> = self.rows.iter().skip(top).map(|row| text(row)).collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    fn apply(&mut self, token: Token) {
        match token {
            Token::Char('\n') => {
                self.row += 1;
                self.col = 0;
            }
            Token::Char('\r') => {
                self.col = 0;
                // Every redraw starts here, including a one-row display's,
                // which doesn't move the cursor up at all.
                self.redrawing = true;
            }
            Token::Char('\t') => {
                for _ in 0..8 - self.col % 8 {
                    self.put(' ');
                }
            }
            // Control characters, and the NULs of a UTF-16 message from wsl.exe,
            // which then reads as plain ASCII.
            Token::Char(c) if c.is_control() => {}
            Token::Char(c) => self.put(c),
            Token::Csi { params, action } => self.csi(&params, action),
            Token::Skip => {}
        }
    }

    fn csi(&mut self, params: &str, action: char) {
        match (params, action) {
            (_, 'A') => {
                let up = params.parse().unwrap_or(1).max(1);
                self.row = self.row.saturating_sub(up);
                self.redrawing = true;
            }
            (_, 'K') => {
                let col = self.col;
                let row = self.current_row();
                match params {
                    "2" => row.clear(),
                    "1" => row.iter_mut().take(col + 1).for_each(|c| *c = ' '),
                    _ => row.truncate(col),
                }
            }
            ("?25", 'l') => self.live_top = Some(self.row),
            ("?25", 'h') => {
                if let Some(top) = self.live_top.take() {
                    self.rows.truncate(top);
                    self.row = top;
                    self.col = 0;
                }
            }
            _ => {}
        }
    }

    fn put(&mut self, c: char) {
        if std::mem::take(&mut self.redrawing) && self.live_top.is_some() {
            self.live_top = Some(self.row);
        }
        let col = self.col;
        let row = self.current_row();
        if row.len() <= col {
            row.resize(col, ' ');
            row.push(c);
        } else {
            row[col] = c;
        }
        self.col += 1;
    }

    fn current_row(&mut self) -> &mut Vec<char> {
        if self.rows.len() <= self.row {
            self.rows.resize(self.row + 1, Vec::new());
        }
        &mut self.rows[self.row]
    }

    /// Hands out the rows above the Live display, or above the cursor when
    /// there's none.
    fn take_finished(&mut self) -> Vec<String> {
        let done = self.live_top.unwrap_or(self.row).min(self.rows.len());
        self.row -= done;
        self.live_top = self.live_top.map(|top| top - done);
        self.rows.drain(..done).map(|row| text(&row)).collect()
    }
}

fn text(row: &[char]) -> String {
    row.iter().collect::<String>().trim_end().to_owned()
}

/// The token at the start of `input` and its length in bytes, or `None` if
/// `input` is empty or ends partway through one.
fn next_token(input: &[u8]) -> Option<(Token, usize)> {
    let first = *input.first()?;
    if first == ESC {
        return escape(input);
    }
    let len = match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        // A stray continuation byte.
        _ => return Some((Token::Skip, 1)),
    };
    let bytes = input.get(..len)?;
    match std::str::from_utf8(bytes) {
        Ok(s) => Some((Token::Char(s.chars().next()?), len)),
        Err(_) => Some((Token::Skip, 1)),
    }
}

fn escape(input: &[u8]) -> Option<(Token, usize)> {
    match *input.get(1)? {
        b'[' => {
            // Parameter and intermediate bytes, then one final byte.
            let end = input[2..]
                .iter()
                .position(|byte| (0x40..=0x7e).contains(byte))?
                + 2;
            let params = String::from_utf8_lossy(&input[2..end]).into_owned();
            let action = char::from(input[end]);
            Some((Token::Csi { params, action }, end + 1))
        }
        // OSC, e.g. a hyperlink: ends with BEL or `ESC \`.
        b']' => {
            let body = &input[2..];
            let end = body.iter().enumerate().find_map(|(i, byte)| match byte {
                0x07 => Some(i + 1),
                &ESC => (body.get(i + 1)? == &b'\\').then_some(i + 2),
                _ => None,
            })?;
            Some((Token::Skip, end + 2))
        }
        _ => Some((Token::Skip, 2)),
    }
}
