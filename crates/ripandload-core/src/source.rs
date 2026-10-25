//! The Sources the app downloads from, and their Quality levels.

use std::fmt;
use std::ops::RangeInclusive;

/// A Source the app has Credentials for. (Tidal and SoundCloud are for later.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    Qobuz,
    Deezer,
}

impl Source {
    /// Which Source a pasted URL belongs to, by its host:
    /// `qobuz.com` or a subdomain (`open.`, `play.`, `www.`), `deezer.com` or a
    /// subdomain, and Deezer's share links on `deezer.page.link` / `dzr.page.link`.
    ///
    /// The URL needs an `http://` or `https://` scheme and something after the
    /// host; scheme and host are case-insensitive. Anything else gives `None`.
    pub fn from_url(url: &str) -> Option<Source> {
        // 🦀 Exercise (ticket 06): make the `from_url` tests below pass.
        let _ = url; // delete this line when you start
        todo!()
    }

    /// streamrip's Quality levels for this Source.
    pub fn quality_levels(self) -> RangeInclusive<u8> {
        match self {
            // 1 = MP3 320, 2 = 16/44.1, 3 = 24/≤96, 4 = 24/>96.
            Source::Qobuz => 1..=4,
            // 0 = MP3 128, 1 = MP3 320, 2 = FLAC.
            Source::Deezer => 0..=2,
        }
    }

    pub fn quality_label(self, level: u8) -> &'static str {
        match (self, level) {
            (Source::Qobuz, 1) => "MP3 320 kbps",
            (Source::Qobuz, 2) => "CD (16-bit / 44.1 kHz)",
            (Source::Qobuz, 3) => "Hi-Res (24-bit / ≤96 kHz)",
            (Source::Qobuz, 4) => "Hi-Res (24-bit / >96 kHz)",
            (Source::Deezer, 0) => "MP3 128 kbps",
            (Source::Deezer, 1) => "MP3 320 kbps",
            (Source::Deezer, 2) => "FLAC",
            _ => "unknown",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::Qobuz => "Qobuz",
            Source::Deezer => "Deezer",
        })
    }
}

/// The Quality levels a Download of `sources` can be overridden to, with labels.
///
/// streamrip's `-q` applies one level to every Source, and a level a Source
/// doesn't have breaks its download. So a mixed Download only offers the levels
/// all its Sources share, with labels that hold for each of them.
pub fn quality_choices(sources: &[Source]) -> Vec<(u8, &'static str)> {
    let Some((&first, rest)) = sources.split_first() else {
        return Vec::new();
    };
    if rest.iter().all(|&source| source == first) {
        return first
            .quality_levels()
            .map(|level| (level, first.quality_label(level)))
            .collect();
    }
    // Qobuz and Deezer share 1 (MP3 320) and 2 (16-bit FLAC).
    vec![(1, "MP3 320 kbps"), (2, "CD quality (16-bit FLAC)")]
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- from_url ---------------------------------------------------------

    fn source(url: &str) -> Option<Source> {
        Source::from_url(url)
    }

    #[test]
    fn qobuz_urls_are_qobuz() {
        for url in [
            "https://open.qobuz.com/album/0060254735180",
            "https://play.qobuz.com/track/12345",
            "https://www.qobuz.com/us-en/album/some-name/abc123",
            "https://qobuz.com/album/1",
        ] {
            assert_eq!(source(url), Some(Source::Qobuz), "{url}");
        }
    }

    #[test]
    fn deezer_urls_are_deezer() {
        for url in [
            "https://www.deezer.com/en/track/3135556",
            "https://deezer.com/album/302127",
            "https://deezer.page.link/urTH8Hw8bvwmJECH6",
            "https://dzr.page.link/abc123",
        ] {
            assert_eq!(source(url), Some(Source::Deezer), "{url}");
        }
    }

    #[test]
    fn http_and_capitals_are_fine() {
        assert_eq!(source("http://open.qobuz.com/track/1"), Some(Source::Qobuz));
        assert_eq!(
            source("HTTPS://Open.Qobuz.COM/track/1"),
            Some(Source::Qobuz)
        );
        assert_eq!(
            source("https://WWW.DEEZER.COM/track/1"),
            Some(Source::Deezer)
        );
    }

    #[test]
    fn other_sources_are_none() {
        assert_eq!(source("https://tidal.com/browse/track/1"), None);
        assert_eq!(source("https://soundcloud.com/artist/track"), None);
        assert_eq!(source("https://www.youtube.com/watch?v=abc"), None);
    }

    #[test]
    fn lookalike_hosts_are_none() {
        assert_eq!(source("https://notqobuz.com/album/1"), None);
        assert_eq!(source("https://qobuz.com.example.org/album/1"), None);
        assert_eq!(source("https://example.org/open.qobuz.com/album/1"), None);
        assert_eq!(source("https://mydeezer.com/track/1"), None);
        assert_eq!(source("https://evil.page.link/abc"), None);
    }

    #[test]
    fn urls_without_a_scheme_or_path_are_none() {
        assert_eq!(source("open.qobuz.com/album/1"), None);
        assert_eq!(source("ftp://open.qobuz.com/album/1"), None);
        assert_eq!(source("https://open.qobuz.com"), None);
        assert_eq!(source("https://open.qobuz.com/"), None);
        assert_eq!(source("https://deezer.page.link?x=1"), None);
        assert_eq!(source(""), None);
        assert_eq!(source("not a url"), None);
    }

    // --- quality_choices -----------------------------------------------------

    #[test]
    fn one_source_offers_its_own_levels_and_labels() {
        assert_eq!(
            quality_choices(&[Source::Deezer]),
            [(0, "MP3 128 kbps"), (1, "MP3 320 kbps"), (2, "FLAC")]
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

    #[test]
    fn mixed_sources_offer_only_the_shared_levels() {
        let expected = [(1, "MP3 320 kbps"), (2, "CD quality (16-bit FLAC)")];
        assert_eq!(quality_choices(&[Source::Qobuz, Source::Deezer]), expected);
        assert_eq!(
            quality_choices(&[Source::Deezer, Source::Qobuz, Source::Deezer]),
            expected
        );
    }

    #[test]
    fn no_sources_offer_nothing() {
        assert_eq!(quality_choices(&[]), []);
    }
}
