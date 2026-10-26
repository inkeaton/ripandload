//! Previews: what a pasted URL points to, looked up before a Download.

use std::fmt;
use std::time::Duration;

use serde_json::Value;
use ureq::{Agent, ResponseExt};

use crate::error::PreviewError;
use crate::source::Source;

const DEEZER_API: &str = "https://api.deezer.com";
const QOBUZ_API: &str = "https://www.qobuz.com/api.json/0.2";
const TIMEOUT: Duration = Duration::from_secs(10);

/// What a URL points to. Only these kinds get a Preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Album,
    Track,
    Playlist,
    Artist,
}

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

/// A URL taken apart into what the APIs need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub source: Source,
    pub kind: Kind,
    pub id: String,
}

/// The Source, kind and ID in a URL's path, for the URL shapes the Source's
/// own sites use. Share links and other shapes give `None`.
pub fn parse(url: &str) -> Option<Link> {
    let source = Source::from_url(url)?;
    let (host, path) = host_and_path(url)?;
    let segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();

    let (kind, id) = match source {
        // open./play.qobuz.com/<kind>/<id>, www.qobuz.com/<locale>/<kind>/<slug>/<id>,
        // where www says `playlists` and `interpreter`.
        Source::Qobuz => match segments[..] {
            [kind, id] => (kind, id),
            [_locale, kind, _slug, id] => (kind, id),
            _ => return None,
        },
        // deezer.com/[<locale>/]<kind>/<id>; share links live on other hosts.
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

/// Whether `url` is a Deezer share link, which only says what it points to
/// once its redirects are followed.
pub fn is_share_link(url: &str) -> bool {
    Source::from_url(url) == Some(Source::Deezer)
        && host_and_path(url).is_some_and(|(host, _)| {
            matches!(
                host.as_str(),
                "deezer.page.link" | "dzr.page.link" | "link.deezer.com"
            )
        })
}

/// The lowercased host and the path, without query or fragment. Only for URLs
/// `Source::from_url` accepted, which checked the scheme and host already.
fn host_and_path(url: &str) -> Option<(String, &str)> {
    let (_, rest) = url.split_once("://")?;
    let rest = &rest[..rest.find(['?', '#']).unwrap_or(rest.len())];
    let (host, path) = rest.split_at(rest.find('/')?);
    Some((host.to_ascii_lowercase(), path))
}

/// The Preview text for an API response about `kind`: `Artist – Title` for
/// albums and tracks, `Name (by Owner)` for playlists, the name for artists.
/// `None` when the response isn't one (Deezer sends errors as JSON too).
pub fn describe(source: Source, kind: Kind, body: &str) -> Option<String> {
    let json: Value = serde_json::from_str(body).ok()?;
    let text = |pointer: &str| {
        json.pointer(pointer)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
    };
    // Qobuz keeps "Remastered", "Deluxe Edition" and the like out of the title.
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

/// What a pasted URL points to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub kind: Kind,
    /// From `describe`, e.g. `Daft Punk – Discovery`.
    pub text: String,
}

/// What Qobuz wants on every request: the `app_id` from the Streamrip config
/// and the user's auth token from Settings. Either can be empty.
#[derive(Debug, Clone, Default)]
pub struct QobuzAccess {
    pub app_id: String,
    pub auth_token: String,
}

/// Looks up what `url` points to. Each request can block for up to
/// `TIMEOUT`, so run it off the UI thread. A Deezer share link is followed to
/// its real URL first.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn link(source: Source, kind: Kind, id: &str) -> Option<Link> {
        Some(Link {
            source,
            kind,
            id: id.to_owned(),
        })
    }

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

    // --- describe ------------------------------------------------------------

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/tests/fixtures/preview/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
    }

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

    #[test]
    fn a_qobuz_version_follows_the_title() {
        let body =
            r#"{"title": "Back in Black", "version": "Remastered", "artist": {"name": "AC/DC"}}"#;
        assert_eq!(
            describe(Source::Qobuz, Kind::Album, body).as_deref(),
            Some("AC/DC – Back in Black (Remastered)")
        );
    }

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
