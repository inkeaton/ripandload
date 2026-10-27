//! The Download tab: pasted URLs, their Previews and a Quality override.
//!
//! The form keeps what the user pasted and splits it into URLs. Each URL
//! shows its Source and, once looked up, its Preview; a URL from no known
//! Source shows a warning and keeps Download disabled. Previews are looked
//! up on threads of their own, once the text box has stopped changing, and
//! kept for the session. The Quality picker offers only the levels every
//! pasted Source has, plus "From Settings".
//!
//! The tab doesn't start or cancel anything. [`show`] reports what the user
//! clicked as an [`Action`], and [`RipAndLoadApp`] starts the run.
//!
//! [`RipAndLoadApp`]: crate::app::RipAndLoadApp

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use ripandload_core::error::{PreviewError, error_chain};
use ripandload_core::preview::{self, Preview, QobuzAccess};
use ripandload_core::source::{self, Source};
use ripandload_core::streamrip_config;

// == Public API ===============================================================

/// The Download tab's form: what the user pasted, its Previews, the Quality.
///
/// Its default is an empty text box, no Previews and Quality from Settings.
#[derive(Debug, Default)]
pub struct DownloadForm {
    /// What the user pasted: URLs separated by spaces or new lines.
    pub pasted: String,
    /// The Quality level for the next Download; `None` uses each Source's
    /// Quality from Settings.
    pub quality: Option<u8>,
    /// The Previews of every URL pasted this session.
    previews: Previews,
}

impl DownloadForm {
    /// Returns the pasted URLs in the order pasted, known Source or not.
    pub fn urls(&self) -> Vec<&str> {
        split_urls(&self.pasted)
    }

    /// Looks up the URLs without a Preview again.
    ///
    /// The app calls it after each run: a Download fills in the Qobuz
    /// `app_id`, and a failed lookup may have been a network blip.
    pub fn retry_previews(&mut self) {
        self.previews.retry();
    }
}

/// What the user clicked in the Download tab this frame.
pub enum Action {
    /// Nothing to start or cancel.
    None,
    /// Start a Download of the pasted URLs.
    Download,
    /// Cancel the current run.
    Cancel,
}

/// Draws the Download tab into `ui` and returns what the user clicked.
///
/// Download is enabled only when no run is going, at least one URL is
/// pasted, and every pasted URL has a known Source. Cancel is enabled only
/// while a run is going.
///
/// # Arguments
///
/// - `ui`: where the tab is drawn.
/// - `form`: the form, edited in place as the user types and picks.
/// - `job_running`: whether a run is going.
/// - `qobuz_auth_token`: the Qobuz auth token from Settings, for looking up
///   Qobuz Previews. When it changes, the URLs without a Preview are looked
///   up again.
///
/// # Returns
///
/// [`Action::Download`] or [`Action::Cancel`] if the user clicked that
/// button this frame, otherwise [`Action::None`].
pub fn show(
    ui: &mut egui::Ui,
    form: &mut DownloadForm,
    job_running: bool,
    qobuz_auth_token: &str,
) -> Action {
    ui.label("Paste one or more Qobuz or Deezer URLs, one per line:");
    let pasted = ui.add(
        egui::TextEdit::multiline(&mut form.pasted)
            .desired_rows(5)
            .desired_width(f32::INFINITY)
            .hint_text("https://open.qobuz.com/album/…"),
    );

    let urls: Vec<(&str, Option<Source>)> = split_urls(&form.pasted)
        .into_iter()
        .map(|url| (url, Source::from_url(url)))
        .collect();
    let sources: Vec<Source> = urls.iter().filter_map(|(_, source)| *source).collect();
    let all_known = urls.iter().all(|(_, source)| source.is_some());

    let known: Vec<&str> = urls
        .iter()
        .filter(|(_, source)| source.is_some())
        .map(|(url, _)| *url)
        .collect();
    form.previews
        .update(ui.ctx(), &known, pasted.changed(), qobuz_auth_token);

    for (url, source) in &urls {
        match source {
            Some(source) => match form.previews.get(url) {
                PreviewState::Ready(preview) => ui
                    .label(format!("✓ {source} {}: {}", preview.kind, preview.text))
                    .on_hover_text(*url),
                PreviewState::LookingUp => ui.label(format!("✓ {source}: {url} · looking up…")),
                PreviewState::Unavailable(text) => ui.label(format!("✓ {source}: {url} · {text}")),
            },
            None => ui.colored_label(
                ui.visuals().warn_fg_color,
                format!("⚠ not a Qobuz or Deezer URL: {url}"),
            ),
        };
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Quality");
        quality_picker(ui, &mut form.quality, &sources);
    });

    ui.add_space(8.0);
    let mut action = Action::None;
    ui.horizontal(|ui| {
        let can_download = !job_running && !urls.is_empty() && all_known;
        if ui
            .add_enabled(can_download, egui::Button::new("Download"))
            .clicked()
        {
            action = Action::Download;
        }
        if ui
            .add_enabled(job_running, egui::Button::new("Cancel"))
            .clicked()
        {
            action = Action::Cancel;
        }
    });
    action
}

// == Previews =================================================================

/// How long the text box must sit still before new URLs are looked up.
///
/// Without it, typing a URL would ask the API about every prefix of it.
const SETTLE_TIME: Duration = Duration::from_millis(500);

/// The Preview of each distinct URL pasted, kept for the session.
///
/// Each lookup runs on a thread of its own and sends its answer back over a
/// channel, which [`Previews::update`] empties every frame.
#[derive(Debug)]
struct Previews {
    /// Every URL asked about so far: `URL -> its lookup's state`.
    by_url: HashMap<String, PreviewState>,
    /// Cloned into each lookup thread, to send its answer back.
    sender: Sender<Finished>,
    /// Where the lookup threads' answers arrive.
    receiver: Receiver<Finished>,
    /// The current lookup round. [`Previews::retry`] starts a new one, so
    /// answers to the lookups it forgot are dropped when they arrive.
    round: u64,
    /// The Qobuz auth token the current answers were looked up with.
    qobuz_auth_token: String,
    /// When the text box last changed; `None` once it has settled.
    edited_at: Option<Instant>,
}

impl Previews {
    /// Takes in finished lookups, then starts one for each new URL.
    ///
    /// New lookups wait until the text box has gone [`SETTLE_TIME`] without
    /// a change; until then, it asks `ctx` for a repaint when that time is
    /// up. A new Qobuz auth token first forgets every URL without a Preview,
    /// so they're looked up again with it.
    ///
    /// # Arguments
    ///
    /// - `ctx`: the UI's context, repainted when a lookup finishes or the
    ///   text box settles.
    /// - `urls`: the pasted URLs with a known Source.
    /// - `edited`: whether the text box changed this frame.
    /// - `qobuz_auth_token`: the Qobuz auth token from Settings.
    fn update(&mut self, ctx: &egui::Context, urls: &[&str], edited: bool, qobuz_auth_token: &str) {
        for (round, url, state) in self.receiver.try_iter() {
            if round == self.round {
                self.by_url.insert(url, state);
            }
        }
        if qobuz_auth_token != self.qobuz_auth_token {
            self.qobuz_auth_token = qobuz_auth_token.to_owned();
            self.retry();
        }

        if edited {
            self.edited_at = Some(Instant::now());
        }
        if let Some(edited_at) = self.edited_at {
            let settled_at = edited_at + SETTLE_TIME;
            let now = Instant::now();
            if now < settled_at {
                ctx.request_repaint_after(settled_at - now);
                return;
            }
            self.edited_at = None;
        }

        for &url in urls {
            if self.by_url.contains_key(url) {
                continue;
            }
            self.by_url.insert(url.to_owned(), PreviewState::LookingUp);
            let (url, auth_token) = (url.to_owned(), qobuz_auth_token.to_owned());
            let (sender, ctx, round) = (self.sender.clone(), ctx.clone(), self.round);
            thread::spawn(move || {
                let state = preview_state(&url, auth_token);
                // Sending fails only once the form is gone, when the app is
                // closing, so the answer has nowhere to go anyway.
                let _ = sender.send((round, url, state));
                ctx.request_repaint();
            });
        }
    }

    /// Forgets every URL without a Preview, so they're looked up again.
    ///
    /// A missing Qobuz auth token or `app_id`, or the network, may have been
    /// fixed since. Lookups still running are left to finish, but their
    /// answers are dropped.
    fn retry(&mut self) {
        self.round += 1;
        self.by_url
            .retain(|_, state| matches!(state, PreviewState::Ready(_)));
    }

    /// Returns `url`'s lookup state, [`PreviewState::LookingUp`] if not asked.
    fn get(&self, url: &str) -> &PreviewState {
        self.by_url.get(url).unwrap_or(&PreviewState::LookingUp)
    }
}

/// No URLs asked about yet, and lookup round 0.
impl Default for Previews {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Previews {
            by_url: HashMap::new(),
            sender,
            receiver,
            round: 0,
            qobuz_auth_token: String::new(),
            edited_at: None,
        }
    }
}

/// Looks up `url`'s Preview, with `auth_token` for a Qobuz URL.
///
/// It blocks until the Source answers, so it runs on a thread of its own.
/// For a Qobuz URL it reads the `app_id` from the Streamrip config, which
/// stays empty until a first Download has written one. A failed lookup is
/// logged at debug level.
///
/// # Arguments
///
/// - `url`: a pasted URL with a known Source.
/// - `auth_token`: the Qobuz auth token from Settings; unused for other
///   Sources.
///
/// # Returns
///
/// - [`PreviewState::Ready`] with the Preview, if the lookup worked.
/// - [`PreviewState::Unavailable`] with the error's message, if the Qobuz
///   `app_id` or auth token is missing, since that message says what to do.
/// - [`PreviewState::Unavailable`] with "no preview", for any other failure.
fn preview_state(url: &str, auth_token: String) -> PreviewState {
    let app_id = match Source::from_url(url) {
        Some(Source::Qobuz) => streamrip_config::default_path()
            .and_then(|config| streamrip_config::qobuz_app_id(&config))
            .unwrap_or_default(),
        _ => String::new(),
    };
    match preview::fetch(url, &QobuzAccess { app_id, auth_token }) {
        Ok(preview) => PreviewState::Ready(preview),
        Err(err) => {
            tracing::debug!("no preview for {url}: {}", error_chain(&err));
            PreviewState::Unavailable(match err {
                // These say what to do about it.
                PreviewError::NoQobuzAppId | PreviewError::NoQobuzToken => err.to_string(),
                _ => "no preview".to_owned(),
            })
        }
    }
}

// == Quality picker ===========================================================

/// Draws the Quality picker into `ui`, editing the Download's `quality`.
///
/// It offers "From Settings" plus the levels every pasted Source has, and
/// is disabled while no pasted URL has a known Source.
///
/// # Arguments
///
/// - `ui`: where the picker is drawn.
/// - `quality`: the chosen level, `None` for "From Settings". Reset to
///   `None` when the pasted Sources no longer offer it.
/// - `sources`: the Source of each pasted URL with a known one.
///
/// # Panics
///
/// Doesn't panic: its one `expect` looks up a level that the reset above it
/// has made sure is among the choices.
fn quality_picker(ui: &mut egui::Ui, quality: &mut Option<u8>, sources: &[Source]) {
    let choices = source::quality_choices(sources);
    // Pasting a URL from another Source can make the chosen level unavailable.
    if quality.is_some_and(|level| !choices.iter().any(|(choice, _)| *choice == level)) {
        *quality = None;
    }
    let label = |level: Option<u8>| match level {
        None => "From Settings".to_owned(),
        Some(level) => {
            let (_, name) = choices
                .iter()
                .find(|(choice, _)| *choice == level)
                .expect("checked above");
            format!("{level}: {name}")
        }
    };

    ui.add_enabled_ui(!choices.is_empty(), |ui| {
        egui::ComboBox::from_id_salt("download_quality")
            .selected_text(label(*quality))
            .show_ui(ui, |ui| {
                ui.selectable_value(quality, None, label(None));
                for &(level, _) in &choices {
                    ui.selectable_value(quality, Some(level), label(Some(level)));
                }
            });
    });
}

// == Helpers ==================================================================

/// Splits `pasted` into URLs at spaces and new lines.
///
/// A free function rather than a method, so [`show`] can borrow just
/// `pasted` while it changes `quality`.
fn split_urls(pasted: &str) -> Vec<&str> {
    pasted.split_whitespace().collect()
}

// == Internal types ===========================================================

/// A lookup's answer: `(round it started in, URL, state)`.
type Finished = (u64, String, PreviewState);

/// Where the lookup of one URL's Preview stands.
#[derive(Debug, Clone)]
enum PreviewState {
    /// The lookup is running, or hasn't started yet.
    LookingUp,
    /// The lookup found this Preview.
    Ready(Preview),
    /// The lookup failed; holds what the URL's line says instead, e.g.
    /// "no preview".
    Unavailable(String),
}
