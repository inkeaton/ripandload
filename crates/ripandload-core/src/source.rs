//! The Sources the app downloads from, and their Quality levels.
//!
//! A pasted URL is matched to its [`Source`] by its host alone. streamrip
//! numbers Quality levels the same for every Source, from 0 (MP3 128 kbps)
//! to 4 (Hi-Res above 96 kHz), and [`quality_label`] names them. Each Source
//! has only some of them, and [`quality_choices`] works out which a
//! Download that mixes Sources can still offer.
//!
//! This module doesn't look inside a URL's path. The Preview, in
//! [`preview`], finds out whether it names an album, a track or a playlist.
//!
//! [`preview`]: crate::preview

use std::fmt;
use std::ops::RangeInclusive;

// == Sources ==================================================================

/// A Source the app can download from.
///
/// Only the Sources the app has Credentials for, so Tidal and SoundCloud
/// aren't among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// Qobuz, reached with a user ID and auth token.
    Qobuz,
    /// Deezer, reached with an ARL.
    Deezer,
}

impl Source {
    /// Returns the Source a pasted URL belongs to, judged by its host.
    ///
    /// The URL needs an `http://` or `https://` scheme and a path after the
    /// host. Scheme and host are case-insensitive; the path isn't checked.
    ///
    /// # Arguments
    ///
    /// - `url`: the pasted URL. Leading whitespace isn't trimmed, so it gives
    ///   `None`.
    ///
    /// # Returns
    ///
    /// - [`Source::Qobuz`] for `qobuz.com` or any of its subdomains (`open.`,
    ///   `play.`, `www.`).
    /// - [`Source::Deezer`] for `deezer.com` or any of its subdomains, and for
    ///   Deezer's share links on `deezer.page.link` and `dzr.page.link`.
    /// - `None` for any other host or scheme, or when the host isn't followed
    ///   by a path longer than `/`.
    pub fn from_url(url: &str) -> Option<Source> {
        let (protocol, url) = url.split_once("://")?;
        if !protocol.eq_ignore_ascii_case("https") && !protocol.eq_ignore_ascii_case("http") {
            return None;
        }

        let host_end = url.find(['/', '?', '#']).unwrap_or(url.len());
        let (host, path) = url.split_at(host_end);
        let host = host.to_ascii_lowercase();

        if !path.starts_with('/') || path.len() < 2 {
            return None;
        };

        let is_qobuz = host == "qobuz.com" || host.ends_with(".qobuz.com");
        let is_deezer = host == "deezer.com"
            || host == "deezer.page.link"
            || host == "dzr.page.link"
            || host.ends_with(".deezer.com");

        if is_qobuz {
            Some(Source::Qobuz)
        } else if is_deezer {
            Some(Source::Deezer)
        } else {
            None
        }
    }

    /// Returns this Source's Quality levels, lowest first.
    ///
    /// The numbers are the ones streamrip's config and its `-q` option use.
    /// [`quality_label`] names each one.
    pub fn quality_levels(self) -> RangeInclusive<u8> {
        match self {
            Source::Qobuz => 1..=4,
            Source::Deezer => 0..=2,
        }
    }
}

/// Shows the Source's name as users know it, e.g. `Qobuz`.
impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::Qobuz => "Qobuz",
            Source::Deezer => "Deezer",
        })
    }
}

// == Quality levels ===========================================================

/// Returns the label shown for a Quality level.
///
/// # Arguments
///
/// - `level`: a Quality level, numbered as streamrip numbers it. The number
///   means the same for every Source.
///
/// # Returns
///
/// The label, e.g. `MP3 320 kbps` or `CD (16-bit / 44.1 kHz)`, or `unknown`
/// when `level` is above 4.
pub fn quality_label(level: u8) -> &'static str {
    match level {
        0 => "MP3 128 kbps",
        1 => "MP3 320 kbps",
        2 => "CD (16-bit / 44.1 kHz)",
        3 => "Hi-Res (24-bit / ≤96 kHz)",
        4 => "Hi-Res (24-bit / >96 kHz)",
        _ => "unknown",
    }
}

/// Returns the Quality levels a Download of `sources` can be overridden to.
///
/// streamrip's `-q` applies one level to every Source, and a level a Source
/// doesn't have breaks that Source's part of the Download. So a Download
/// that mixes Sources only offers the levels they all share.
///
/// # Arguments
///
/// - `sources`: the Source of each pasted URL, in any order, repeats
///   included.
///
/// # Returns
///
/// `(level, label)` pairs, lowest level first, labeled by
/// [`quality_label`]: the levels that all of `sources`'
/// [`Source::quality_levels`] share. Nothing when `sources` is empty or
/// shares no level.
pub fn quality_choices(sources: &[Source]) -> Vec<(u8, &'static str)> {
    let Some((&first, rest)) = sources.split_first() else {
        return Vec::new();
    };
    let (mut low, mut high) = first.quality_levels().into_inner();
    for source in rest {
        let (source_low, source_high) = source.quality_levels().into_inner();
        low = low.max(source_low);
        high = high.min(source_high);
    }
    return (low..=high)
        .map(|level| (level, quality_label(level)))
        .collect();
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns the Source [`Source::from_url`] finds for `url`.
    fn source_of(url: &str) -> Option<Source> {
        Source::from_url(url)
    }

    // == Matching URLs ========================================================

    /// Qobuz URLs, on the bare domain or a subdomain, are Qobuz.
    #[test]
    fn qobuz_urls_are_qobuz() {
        for url in [
            "https://open.qobuz.com/album/0060254735180",
            "https://play.qobuz.com/track/12345",
            "https://www.qobuz.com/us-en/album/some-name/abc123",
            "https://qobuz.com/album/1",
        ] {
            assert_eq!(source_of(url), Some(Source::Qobuz), "{url}");
        }
    }

    /// Deezer URLs, share links included, are Deezer.
    #[test]
    fn deezer_urls_are_deezer() {
        for url in [
            "https://www.deezer.com/en/track/3135556",
            "https://deezer.com/album/302127",
            "https://deezer.page.link/urTH8Hw8bvwmJECH6",
            "https://dzr.page.link/abc123",
        ] {
            assert_eq!(source_of(url), Some(Source::Deezer), "{url}");
        }
    }

    /// An `http` scheme, and capitals in the scheme or host, still match.
    #[test]
    fn http_and_capitals_are_fine() {
        assert_eq!(
            source_of("http://open.qobuz.com/track/1"),
            Some(Source::Qobuz)
        );
        assert_eq!(
            source_of("HTTPS://Open.Qobuz.COM/track/1"),
            Some(Source::Qobuz)
        );
        assert_eq!(
            source_of("https://WWW.DEEZER.COM/track/1"),
            Some(Source::Deezer)
        );
    }

    /// Tidal, SoundCloud and other sites give `None`.
    #[test]
    fn other_sources_are_none() {
        assert_eq!(source_of("https://tidal.com/browse/track/1"), None);
        assert_eq!(source_of("https://soundcloud.com/artist/track"), None);
        assert_eq!(source_of("https://www.youtube.com/watch?v=abc"), None);
    }

    /// A host that only contains a Source's domain gives `None`.
    #[test]
    fn lookalike_hosts_are_none() {
        assert_eq!(source_of("https://notqobuz.com/album/1"), None);
        assert_eq!(source_of("https://qobuz.com.example.org/album/1"), None);
        assert_eq!(
            source_of("https://example.org/open.qobuz.com/album/1"),
            None
        );
        assert_eq!(source_of("https://mydeezer.com/track/1"), None);
        assert_eq!(source_of("https://evil.page.link/abc"), None);
    }

    /// A URL without an `http` or `https` scheme, or a path, gives `None`.
    #[test]
    fn urls_without_a_scheme_or_path_are_none() {
        assert_eq!(source_of("open.qobuz.com/album/1"), None);
        assert_eq!(source_of("ftp://open.qobuz.com/album/1"), None);
        assert_eq!(source_of("https://open.qobuz.com"), None);
        assert_eq!(source_of("https://open.qobuz.com/"), None);
        assert_eq!(source_of("https://deezer.page.link?x=1"), None);
        assert_eq!(source_of(""), None);
        assert_eq!(source_of("not a url"), None);
    }

    // == Quality choices ======================================================

    /// One Source, even repeated, offers all its own levels and labels.
    #[test]
    fn one_source_offers_its_own_levels_and_labels() {
        assert_eq!(
            quality_choices(&[Source::Deezer]),
            [
                (0, "MP3 128 kbps"),
                (1, "MP3 320 kbps"),
                (2, "CD (16-bit / 44.1 kHz)"),
            ]
        );
        assert_eq!(
            quality_choices(&[Source::Qobuz, Source::Qobuz]),
            [
                (1, "MP3 320 kbps"),
                (2, "CD (16-bit / 44.1 kHz)"),
                (3, "Hi-Res (24-bit / ≤96 kHz)"),
                (4, "Hi-Res (24-bit / >96 kHz)"),
            ]
        );
    }

    /// Mixed Sources, in any order, offer only the levels they share.
    #[test]
    fn mixed_sources_offer_only_the_shared_levels() {
        let expected = [(1, "MP3 320 kbps"), (2, "CD (16-bit / 44.1 kHz)")];
        assert_eq!(quality_choices(&[Source::Qobuz, Source::Deezer]), expected);
        assert_eq!(
            quality_choices(&[Source::Deezer, Source::Qobuz, Source::Deezer]),
            expected
        );
    }

    /// No Sources offer no levels.
    #[test]
    fn no_sources_offer_nothing() {
        assert_eq!(quality_choices(&[]), []);
    }
}
