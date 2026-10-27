//! Previews of pasted URLs, looked up before a Download.
//!
//! A Preview says what a pasted URL points to: its kind, and a line of text
//! such as `Daft Punk – Discovery`. Getting one takes three steps, each its
//! own function so that reading URLs and answers is tested without the
//! network:
//!
//! 1. [`parse`] reads the Source, kind and ID out of the URL. A Deezer share
//!    link has none until its redirects are followed.
//! 2. [`fetch`] asks the Source's public API about that ID.
//! 3. [`describe`] turns the API's JSON answer into the Preview's text.
//!
//! A Preview is only information. Failing to get one never blocks a
//! Download, and this module never starts one: streamrip reads the URL
//! itself.

use std::fmt;
use std::time::Duration;

use serde_json::Value;
use ureq::{Agent, ResponseExt};

use crate::error::PreviewError;
use crate::source::Source;

// == Previews =================================================================

/// The kind of item a URL points to. Only these kinds get a Preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An album.
    Album,
    /// A single track.
    Track,
    /// A playlist.
    Playlist,
    /// An artist.
    Artist,
}

/// Shows the kind the way both APIs spell it in requests, e.g. `album`.
impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Album => "album",
            Kind::Track => "track",
            Kind::Playlist => "playlist",
            Kind::Artist => "artist",
        })
    }
}

/// What a pasted URL points to, as [`fetch`] found it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// What the URL points to.
    pub kind: Kind,
    /// One line naming it, from [`describe`], e.g. `Daft Punk – Discovery`.
    pub text: String,
}

// == Reading URLs =============================================================

/// A URL taken apart into what the APIs need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The Source whose site the URL is on.
    pub source: Source,
    /// What the URL points to.
    pub kind: Kind,
    /// The Source's ID for it, letters and digits only.
    pub id: String,
}

/// Reads the Source, kind and ID out of the path of `url`.
///
/// Only the URL shapes of the Source's own sites are read. The query and
/// fragment are ignored.
///
/// # Returns
///
/// The [`Link`], or `None` for:
///
/// - a Deezer share link (see [`is_share_link`]), or any other Deezer host
///   than `www.deezer.com` and `deezer.com`;
/// - a path of any other shape than those below;
/// - a kind with no Preview;
/// - an ID that isn't plain letters and digits;
/// - a URL of no Source at all.
///
/// # Format
///
/// ```text
/// https://open.qobuz.com/<kind>/<id>                (also play.qobuz.com)
/// https://www.qobuz.com/<locale>/<kind>/<slug>/<id>
/// https://www.deezer.com/[<locale>/]<kind>/<id>     (also deezer.com)
/// ```
///
/// where `<kind>` is `album`, `track`, `playlist` or `artist`, and
/// www.qobuz.com says `playlists` and `interpreter` for the last two.
pub fn parse(url: &str) -> Option<Link> {
    let source = Source::from_url(url)?;
    let (host, path) = host_and_path(url)?;
    let segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();

    let (kind, id) = match source {
        Source::Qobuz => match segments[..] {
            [kind, id] => (kind, id),
            [_locale, kind, _slug, id] => (kind, id),
            _ => return None,
        },
        // The locale is optional. Share links live on other hosts.
        Source::Deezer if host == "deezer.com" || host == "www.deezer.com" => match segments[..] {
            [kind, id] | [_, kind, id] => (kind, id),
            _ => return None,
        },
        Source::Deezer => return None,
    };

    let kind = match kind {
        "album" => Kind::Album,
        "track" => Kind::Track,
        "playlist" | "playlists" => Kind::Playlist,
        "artist" | "interpreter" => Kind::Artist,
        _ => return None,
    };
    // The ID goes into an API URL, so nothing but letters and digits.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(Link {
        source,
        kind,
        id: id.to_owned(),
    })
}

/// Returns whether `url` is a Deezer share link.
///
/// A share link (`deezer.page.link`, `dzr.page.link`, `link.deezer.com`)
/// only says what it points to once its redirects are followed, which
/// [`fetch`] does.
pub fn is_share_link(url: &str) -> bool {
    Source::from_url(url) == Some(Source::Deezer)
        && host_and_path(url).is_some_and(|(host, _)| {
            matches!(
                host.as_str(),
                "deezer.page.link" | "dzr.page.link" | "link.deezer.com"
            )
        })
}

/// Splits `url` into its lowercased host and its path.
///
/// It is only meant for URLs [`Source::from_url`] accepted, which has
/// checked the scheme and host already.
///
/// # Returns
///
/// The host and the path, without query or fragment, or `None` when `url`
/// has no `://` or no path.
fn host_and_path(url: &str) -> Option<(String, &str)> {
    let (_, rest) = url.split_once("://")?;
    let rest = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    let (host, path) = rest.split_at(rest.find('/')?);
    Some((host.to_ascii_lowercase(), path))
}

// == Describing answers =======================================================

/// Returns the Preview's text for a Source's API answer about a `kind`.
///
/// Qobuz's answers keep a version, such as "Remastered" or "Deluxe
/// Edition", out of the title, so it is added back in brackets after it.
///
/// # Arguments
///
/// - `source`: the Source that answered, which decides where the names are.
/// - `kind`: what was asked about.
/// - `body`: the answer, as JSON text.
///
/// # Returns
///
/// - `Artist – Title` for an album or a track.
/// - `Name (by Owner)` for a playlist.
/// - The name for an artist.
/// - `None` when `body` isn't JSON, is an error sent as JSON (as Deezer
///   does), or lacks one of the names above.
pub fn describe(source: Source, kind: Kind, body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    let text = |pointer: &str| {
        json.pointer(pointer)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
    };
    let title = |key: &str| {
        let title = text(key)?;
        Some(match text("/version") {
            Some(version) if source == Source::Qobuz => format!("{title} ({version})"),
            _ => title.to_owned(),
        })
    };

    match (source, kind) {
        (Source::Deezer, Kind::Album | Kind::Track) => {
            Some(format!("{} – {}", text("/artist/name")?, title("/title")?))
        }
        (Source::Qobuz, Kind::Album) => {
            Some(format!("{} – {}", text("/artist/name")?, title("/title")?))
        }
        (Source::Qobuz, Kind::Track) => Some(format!(
            "{} – {}",
            text("/performer/name")?,
            title("/title")?
        )),
        (Source::Deezer, Kind::Playlist) => Some(format!(
            "{} (by {})",
            text("/title")?,
            text("/creator/name")?
        )),
        (Source::Qobuz, Kind::Playlist) => {
            Some(format!("{} (by {})", text("/name")?, text("/owner/name")?))
        }
        (_, Kind::Artist) => text("/name").map(str::to_owned),
    }
}

// == Fetching =================================================================

/// Base URL of Deezer's public API, which needs no login.
const DEEZER_API: &str = "https://api.deezer.com";
/// Base URL of Qobuz's API, which needs an app ID and an auth token.
const QOBUZ_API: &str = "https://www.qobuz.com/api.json/0.2";
/// How long one request may take in all before it is given up.
const TIMEOUT: Duration = Duration::from_secs(10);

/// What Qobuz wants on every request. Either field can be empty.
///
/// The auth token is one of the Credentials. The app ID isn't: it is
/// streamrip's own, so it comes from the Streamrip config.
#[derive(Debug, Clone, Default)]
pub struct QobuzAccess {
    /// The app ID, read from the Streamrip config once streamrip has filled
    /// it in.
    pub app_id: String,
    /// The user's auth token, from Settings.
    pub auth_token: String,
}

/// Returns the [`Preview`] of `url`, asking its Source's API.
///
/// A Deezer share link is followed to its real URL first, which takes one
/// more request. Each request blocks for up to 10 seconds, so call this off
/// the UI thread.
///
/// # Arguments
///
/// - `url`: the pasted URL.
/// - `qobuz`: what Qobuz wants on every request; unused for Deezer.
///
/// # Errors
///
/// - [`PreviewError::Unsupported`] if [`parse`] finds no [`Link`] in `url`
///   and it isn't a share link either.
/// - [`PreviewError::UnreadableShareLink`] if `parse` finds no Link in the
///   URL a share link led to.
/// - [`PreviewError::NoQobuzAppId`] or [`PreviewError::NoQobuzToken`] if a
///   Qobuz URL is pasted while `qobuz` lacks either.
/// - [`PreviewError::Request`] if a request failed or its answer couldn't be
///   read.
/// - [`PreviewError::Unexpected`] if [`describe`] found no Preview in the
///   answer.
pub fn fetch(url: &str, qobuz: &QobuzAccess) -> Result<Preview, PreviewError> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();

    let link = match parse(url) {
        Some(link) => link,
        None if is_share_link(url) => {
            let response = agent.get(url).call().map_err(|err| PreviewError::Request {
                from: Source::Deezer,
                err: Box::new(err),
            })?;
            parse(&response.get_uri().to_string()).ok_or(PreviewError::UnreadableShareLink)?
        }
        None => return Err(PreviewError::Unsupported),
    };

    let request = match link.source {
        Source::Deezer => agent.get(format!("{DEEZER_API}/{}/{}", link.kind, link.id)),
        Source::Qobuz => {
            if qobuz.app_id.is_empty() {
                return Err(PreviewError::NoQobuzAppId);
            }
            if qobuz.auth_token.is_empty() {
                return Err(PreviewError::NoQobuzToken);
            }
            agent
                .get(format!("{QOBUZ_API}/{}/get", link.kind))
                .query(format!("{}_id", link.kind), &link.id)
                .header("X-App-Id", &qobuz.app_id)
                .header("X-User-Auth-Token", &qobuz.auth_token)
        }
    };
    let request_error = |err| PreviewError::Request {
        from: link.source,
        err: Box::new(err),
    };
    let body = request
        .call()
        .map_err(request_error)?
        .body_mut()
        .read_to_string()
        .map_err(request_error)?;
    let text = describe(link.source, link.kind, &body)
        .ok_or(PreviewError::Unexpected { from: link.source })?;
    Ok(Preview {
        kind: link.kind,
        text,
    })
}

// == Tests ====================================================================

#[cfg(test)]
#[allow(clippy::missing_panics_doc, reason = "a panic is how a test fails")]
mod tests {
    use super::*;

    // == Fixtures =============================================================

    /// Returns `Some` of the [`Link`] made of `source`, `kind` and `id`.
    fn link(source: Source, kind: Kind, id: &str) -> Option<Link> {
        Some(Link {
            source,
            kind,
            id: id.to_owned(),
        })
    }

    /// Returns the API answer saved in `tests/fixtures/preview/<name>`.
    ///
    /// # Panics
    ///
    /// Panics, naming the path, if the file can't be read.
    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/tests/fixtures/preview/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
    }

    // == Reading URLs =========================================================

    /// Each Qobuz URL shape gives its kind and ID, ignoring query and slash.
    #[test]
    fn qobuz_urls_give_their_kind_and_id() {
        use Kind::*;
        for (url, kind, id) in [
            (
                "https://open.qobuz.com/album/0060254735180",
                Album,
                "0060254735180",
            ),
            ("https://play.qobuz.com/track/24393117", Track, "24393117"),
            (
                "https://play.qobuz.com/playlist/1141084",
                Playlist,
                "1141084",
            ),
            ("https://open.qobuz.com/artist/36819", Artist, "36819"),
            (
                "https://www.qobuz.com/us-en/album/true-colors-zedd/0060254735180",
                Album,
                "0060254735180",
            ),
            (
                "https://www.qobuz.com/us-en/playlists/the-beatles/1141084",
                Playlist,
                "1141084",
            ),
            (
                "https://www.qobuz.com/gb-en/interpreter/daft-punk/36819",
                Artist,
                "36819",
            ),
            ("https://open.qobuz.com/album/abc123/", Album, "abc123"),
            (
                "https://open.qobuz.com/album/abc123?utm_source=x#top",
                Album,
                "abc123",
            ),
        ] {
            assert_eq!(parse(url), link(Source::Qobuz, kind, id), "{url}");
        }
    }

    /// Each Deezer URL shape gives its kind and ID, with or without locale.
    #[test]
    fn deezer_urls_give_their_kind_and_id() {
        use Kind::*;
        for (url, kind, id) in [
            ("https://www.deezer.com/en/album/302127", Album, "302127"),
            ("https://deezer.com/track/3135556", Track, "3135556"),
            (
                "https://www.deezer.com/fr/playlist/908622995",
                Playlist,
                "908622995",
            ),
            (
                "https://www.deezer.com/artist/27?host=123&utm_campaign=x",
                Artist,
                "27",
            ),
        ] {
            assert_eq!(parse(url), link(Source::Deezer, kind, id), "{url}");
        }
    }

    /// Share links, other kinds and shapes, and other sites give no Link.
    #[test]
    fn other_urls_have_no_link() {
        for url in [
            // Share links are followed to a real URL first.
            "https://deezer.page.link/urTH8Hw8bvwmJECH6",
            "https://link.deezer.com/s/30ABCdef",
            // Kinds without a Preview.
            "https://open.qobuz.com/label/12345",
            "https://www.qobuz.com/us-en/interpreter/daft-punk/download-streaming-albums",
            "https://www.deezer.com/en/show/12345",
            // Missing or odd IDs.
            "https://open.qobuz.com/album/",
            "https://www.deezer.com/en/album",
            "https://www.deezer.com/en/album/..%2F..",
            "https://www.deezer.com/en/album/302127/extra/bits",
            // Not a Source at all.
            "https://tidal.com/browse/album/1",
            "not a url",
        ] {
            assert_eq!(parse(url), None, "{url}");
        }
    }

    /// Only Deezer's share hosts are share links, in any case.
    #[test]
    fn only_deezer_share_links_are_share_links() {
        for url in [
            "https://deezer.page.link/urTH8Hw8bvwmJECH6",
            "https://dzr.page.link/abc123",
            "https://link.deezer.com/s/30ABCdef",
            "HTTPS://Link.Deezer.com/s/30ABCdef",
        ] {
            assert!(is_share_link(url), "{url}");
        }
        for url in [
            "https://www.deezer.com/en/album/302127",
            "https://www.deezer.com/en/show/12345",
            "https://www.deezer.com/en/alb",
            "https://open.qobuz.com/album/1",
            "https://evil.page.link/abc",
            "not a url",
        ] {
            assert!(!is_share_link(url), "{url}");
        }
    }

    // == Describing answers ===================================================

    /// Deezer's answers for each kind describe what was pasted.
    #[test]
    fn deezer_responses_describe_what_was_pasted() {
        use Kind::*;
        for (file, kind, text) in [
            ("deezer-album.json", Album, "Daft Punk – Discovery"),
            (
                "deezer-track.json",
                Track,
                "Daft Punk – Harder, Better, Faster, Stronger",
            ),
            (
                "deezer-playlist.json",
                Playlist,
                "En mode 60 (by Alexandre - Pop & Hits Editor)",
            ),
            ("deezer-artist.json", Artist, "Daft Punk"),
        ] {
            let body = fixture(file);
            assert_eq!(
                describe(Source::Deezer, kind, &body).as_deref(),
                Some(text),
                "{file}"
            );
        }
    }

    /// Qobuz's answers for each kind describe what was pasted.
    #[test]
    fn qobuz_responses_describe_what_was_pasted() {
        use Kind::*;
        for (file, kind, text) in [
            ("qobuz-album.json", Album, "Zedd – True Colors"),
            ("qobuz-track.json", Track, "Zedd – Addicted To A Memory"),
            (
                "qobuz-playlist.json",
                Playlist,
                "The Beatles (by Qobuz France)",
            ),
            ("qobuz-artist.json", Artist, "Daft Punk"),
        ] {
            let body = fixture(file);
            assert_eq!(
                describe(Source::Qobuz, kind, &body).as_deref(),
                Some(text),
                "{file}"
            );
        }
    }

    /// A Qobuz version follows the title in brackets.
    #[test]
    fn a_qobuz_version_follows_the_title() {
        let body =
            r#"{"title": "Back in Black", "version": "Remastered", "artist": {"name": "AC/DC"}}"#;
        assert_eq!(
            describe(Source::Qobuz, Kind::Album, body).as_deref(),
            Some("AC/DC – Back in Black (Remastered)")
        );
    }

    /// Errors, answers of another kind and non-JSON describe nothing.
    #[test]
    fn errors_and_other_shapes_describe_nothing() {
        let error = fixture("deezer-error.json");
        assert_eq!(describe(Source::Deezer, Kind::Album, &error), None);
        // An artist response read as an album has no title.
        let artist = fixture("deezer-artist.json");
        assert_eq!(describe(Source::Deezer, Kind::Album, &artist), None);
        assert_eq!(describe(Source::Qobuz, Kind::Track, "not json"), None);
    }
}
