//! How the GUI words counts and times.

use std::time::Instant;

/// Returns `count` and `what`, with an "s" unless `count` is 1.
///
/// # Format
///
/// ```text
/// 1 music file
/// 3 music files
/// ```
pub fn plural(count: usize, what: &str) -> String {
    match count {
        1 => format!("1 {what}"),
        n => format!("{n} {what}s"),
    }
}

/// Returns how long ago `at` was, rounded down.
///
/// # Format
///
/// ```text
/// just now
/// 5 min ago
/// 2 h ago
/// ```
pub fn ago(at: Instant) -> String {
    match at.elapsed().as_secs() / 60 {
        0 => "just now".to_owned(),
        minutes @ 1..60 => format!("{minutes} min ago"),
        minutes => format!("{} h ago", minutes / 60),
    }
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;
    use std::time::Duration;

    // == Fixtures =============================================================

    /// Returns the moment `age` ago.
    ///
    /// # Panics
    ///
    /// Panics if the clock started less than `age` ago.
    fn ago_of(age: Duration) -> String {
        ago(Instant::now().checked_sub(age).unwrap())
    }

    // == Counts ===============================================================

    /// One takes no "s"; zero and many do.
    #[test]
    fn plural_adds_an_s_unless_there_is_one() {
        assert_eq!(plural(0, "file"), "0 files");
        assert_eq!(plural(1, "file"), "1 file");
        assert_eq!(plural(3, "music file"), "3 music files");
    }

    // == Times ================================================================

    /// Under a minute is just now.
    #[test]
    fn under_a_minute_is_just_now() {
        assert_eq!(ago_of(Duration::from_secs(59)), "just now");
    }

    /// Minutes are rounded down, up to the hour.
    #[test]
    fn minutes_are_rounded_down() {
        assert_eq!(ago_of(Duration::from_secs(60)), "1 min ago");
        assert_eq!(ago_of(Duration::from_secs(59 * 60 + 59)), "59 min ago");
    }

    /// From an hour on, it counts hours, rounded down.
    #[test]
    fn hours_are_rounded_down() {
        assert_eq!(ago_of(Duration::from_secs(60 * 60)), "1 h ago");
        assert_eq!(ago_of(Duration::from_secs(150 * 60)), "2 h ago");
    }
}
