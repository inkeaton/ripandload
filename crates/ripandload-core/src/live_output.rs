//! A Download's output, as a terminal would show it.
//!
//! streamrip draws its progress bars with rich's `Live` display, which
//! redraws them in place. [`LiveOutput`] replays the output on a grid of
//! rows, the way a terminal would, and splits it in two: finished lines,
//! which never change again and go to the log, and the live rows, which show
//! progress and never reach the log.
//!
//! This module reads no process and writes no log. [`JobRun`] feeds it a
//! Download's output and decides where the lines go.
//!
//! # How rich draws
//!
//! rich hides the cursor and draws a few rows. Ten times a second it goes
//! back to the start of the row (`\r`), moves the cursor up over the others
//! (`ESC[1A`), clears each (`ESC[2K`) and draws them again. A log message is
//! printed the same way, in place of the bars, with the bars drawn again
//! below it. So a row above where the last redraw started writing will never
//! change again: that's a finished line. The rows from there down are the
//! live rows, and when the cursor shows again (`ESC[?25h`) the display is
//! over and they're dropped.
//!
//! # What is understood
//!
//! - `\r` and `\n`, which move the cursor.
//! - `ESC[<n>A`, which moves the cursor up.
//! - `ESC[<n>K`, which clears all or part of a row.
//! - `ESC[?25l` and `ESC[?25h`, which hide and show the cursor, and so start
//!   and end the Live display.
//!
//! Colors, hyperlinks and anything else are skipped.
//!
//! [`JobRun`]: crate::job_run::JobRun

/// The byte that starts an escape sequence.
const ESC: u8 = 0x1b;

/// Bytes kept back for an unfinished escape sequence before it's skipped.
///
/// Longer than any escape sequence rich writes; a hyperlink is a few hundred
/// bytes.
const MAX_PENDING: usize = 4096;

// == Live output ==============================================================

/// One stream of a Download's output, replayed on a grid of rows.
///
/// Feed it the output with [`LiveOutput::feed`], in chunks of any size, then
/// call [`LiveOutput::finish`] once the process has exited. In between,
/// [`LiveOutput::live_rows`] shows the Live display. A `default()` value has
/// an empty grid, with the cursor at the top and the Live display off.
#[derive(Debug, Default)]
pub struct LiveOutput {
    /// The rows not yet handed out as finished lines, one `char` per column.
    rows: Vec<Vec<char>>,
    /// The cursor's row, as an index into `rows`.
    row: usize,
    /// The cursor's column, as an index into its row.
    col: usize,
    /// While the Live display is on: the row it starts at.
    live_top: Option<usize>,
    /// A redraw began (`\r`, then the cursor going up over the old rows) and
    /// hasn't written anything yet. Until it does, a chunk may have ended
    /// partway up, so `live_top` stays put.
    redrawing: bool,
    /// The start of an escape sequence or character the last chunk cut off,
    /// read again in front of the next chunk.
    pending: Vec<u8>,
}

impl LiveOutput {
    // == Feeding output =======================================================

    /// Takes the next bytes of output and returns the lines now finished.
    ///
    /// A chunk may end partway through an escape sequence or a character;
    /// that start is kept until the next chunk. An escape sequence still
    /// unfinished after 4 KiB never ends, so its `ESC` is skipped and the
    /// rest read as text.
    ///
    /// # Arguments
    ///
    /// - `bytes`: the next chunk of output, of any size.
    ///
    /// # Returns
    ///
    /// The rows now above the Live display, or above the cursor when the
    /// display is off, in order and without trailing spaces. Empty when no
    /// row was finished.
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

    /// Returns what's left once the process has exited, and starts over.
    ///
    /// A Live display that never ended, e.g. after Cancel, is dropped, so its
    /// progress never reaches the log, and so is a cut-off escape sequence or
    /// character left from the last chunk. Afterwards the value is as new.
    ///
    /// # Returns
    ///
    /// The rows not yet handed out, in order and without trailing spaces,
    /// leaving out the Live display and any empty rows at the end.
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

    /// Returns the rows the Live display shows right now.
    ///
    /// # Returns
    ///
    /// The rows from the top of the Live display down, without trailing
    /// spaces or empty rows at the end. Empty when the display is off.
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

    /// Removes and returns the finished rows, as [`LiveOutput::feed`] returns.
    ///
    /// The cursor and `live_top` move up by as many rows, so they still point
    /// at the same rows.
    ///
    /// Rows at or below the cursor stay, even when the cursor is above the
    /// top of the Live display. Cursor up past the rows already handed out
    /// stops at the first row kept, so it never rewrites a row already
    /// returned.
    ///
    /// # Panics
    ///
    /// Doesn't panic: `done` is at most `row`, so the cursor can't go below
    /// 0, and at most `live_top` when there is one, so `live_top` can't
    /// either.
    fn take_finished(&mut self) -> Vec<String> {
        let done = self
            .live_top
            .unwrap_or(self.row)
            .min(self.rows.len())
            .min(self.row);
        self.row -= done;
        self.live_top = self.live_top.map(|top| top - done);
        self.rows.drain(..done).map(|row| text(&row)).collect()
    }

    // == Drawing on the grid ==================================================

    /// Draws `token` on the grid and moves the cursor, as a terminal would.
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
            // Other control characters are dropped, including the NULs of a
            // UTF-16 message from wsl.exe, which then reads as plain ASCII.
            Token::Char(c) if c.is_control() => {}
            Token::Char(c) => self.put(c),
            Token::Csi { params, action } => self.csi(&params, action),
            Token::Skip => {}
        }
    }

    /// Carries out the CSI sequence `ESC [ params action`.
    ///
    /// Understands cursor up (`A`), clearing in the row (`K`), and hiding and
    /// showing the cursor (`?25l`, `?25h`), which start and end the Live
    /// display. Any other sequence does nothing.
    ///
    /// # Arguments
    ///
    /// - `params`: the parameter bytes, e.g. the `2` in `ESC[2K` or the `?25`
    ///   in `ESC[?25l`. Empty or unreadable means the default, 1 row up.
    /// - `action`: the final byte, which says what to do.
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

    /// Writes `c` at the cursor and moves the cursor one column right.
    ///
    /// A row shorter than the cursor's column is padded with spaces. The
    /// first character a redraw writes marks where the Live display now
    /// starts.
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

    /// Returns the cursor's row, adding empty rows down to it if needed.
    fn current_row(&mut self) -> &mut Vec<char> {
        if self.rows.len() <= self.row {
            self.rows.resize(self.row + 1, Vec::new());
        }
        &mut self.rows[self.row]
    }
}

// == Tokens ===================================================================

/// One step of the output: a character or an escape sequence.
enum Token {
    /// A character, control characters included.
    Char(char),
    /// A CSI sequence, `ESC [ params action`.
    Csi {
        /// The parameter and intermediate bytes, e.g. `2` or `?25`.
        params: String,
        /// The final byte, which says what to do, e.g. `K`.
        action: char,
    },
    /// Any other escape sequence, or a byte that isn't UTF-8.
    Skip,
}

/// Reads the token at the start of `input`.
///
/// # Returns
///
/// The token and its length in bytes, or `None` if `input` is empty or ends
/// partway through a token. A byte that can't be read as UTF-8 is a
/// one-byte [`Token::Skip`].
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

/// Reads the escape sequence at the start of `input`, which starts with `ESC`.
///
/// # Returns
///
/// The token and its length in bytes:
///
/// - a [`Token::Csi`] for `ESC [`;
/// - a [`Token::Skip`] for an OSC sequence (`ESC ]`, such as a hyperlink),
///   up to its closing BEL or `ESC \`;
/// - a two-byte [`Token::Skip`] for any other sequence;
/// - `None` when `input` ends before the sequence does.
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

// == Helpers ==================================================================

/// Returns `row` as a string, without trailing spaces.
fn text(row: &[char]) -> String {
    row.iter().collect::<String>().trim_end().to_owned()
}
