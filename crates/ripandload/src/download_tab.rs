//! The Download form: pasted URLs, their Previews and an optional Quality override.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use ripandload_core::error::{PreviewError, error_chain};
use ripandload_core::preview::{self, Preview, QobuzAccess};
use ripandload_core::source::{self, Source};
use ripandload_core::streamrip_config;

#[derive(Debug, Default)]
pub struct DownloadForm {
    /// What the user pasted: URLs separated by spaces or new lines.
    pub pasted: String,
    /// `None` uses each Source's Quality from Settings.
    pub quality: Option<u8>,
    previews: Previews,
}

/// How long the text box must sit still before new URLs are looked up, so
/// typing a URL doesn't ask the API about every prefix of it.
const SETTLE_TIME: Duration = Duration::from_millis(500);

/// The Preview of each distinct URL, kept for the session.
#[derive(Debug)]
struct Previews {
    by_url: HashMap<String, PreviewState>,
    sender: Sender<Finished>,
    receiver: Receiver<Finished>,
    /// Bumped by `retry`, so answers to the lookups it dropped are ignored.
    round: u64,
    /// The Qobuz auth token the cached answers were looked up with.
    qobuz_auth_token: String,
    /// When the text box last changed.
    edited_at: Option<Instant>,
}

/// A lookup's answer: the round it started in, its URL and its result.
type Finished = (u64, String, PreviewState);

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

#[derive(Debug, Clone)]
enum PreviewState {
    LookingUp,
    Ready(Preview),
    /// What the line says instead, e.g. "no preview".
    Unavailable(String),
}

impl Previews {
    /// Takes in finished lookups, and once the text box has settled starts one
    /// for each URL not seen before.
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
                // The form is gone only when the app is closing.
                let _ = sender.send((round, url, state));
                ctx.request_repaint();
            });
        }
    }

    /// Forgets every URL without a Preview, so they're looked up again: a
    /// missing token or `app_id`, or the network, may have been fixed since.
    fn retry(&mut self) {
        self.round += 1;
        self.by_url
            .retain(|_, state| matches!(state, PreviewState::Ready(_)));
    }

    fn get(&self, url: &str) -> &PreviewState {
        self.by_url.get(url).unwrap_or(&PreviewState::LookingUp)
    }
}

/// Runs on its own thread: `fetch` blocks.
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

impl DownloadForm {
    pub fn urls(&self) -> Vec<&str> {
        split_urls(&self.pasted)
    }

    /// Looks up the URLs without a Preview again. A Download fills in the
    /// Qobuz `app_id`, and a failed lookup may have been a network blip.
    pub fn retry_previews(&mut self) {
        self.previews.retry();
    }
}

/// A free function rather than a method, so `show` can borrow just `pasted`
/// while it changes `quality`.
fn split_urls(pasted: &str) -> Vec<&str> {
    pasted.split_whitespace().collect()
}

/// What the user asked for this frame.
pub enum Action {
    None,
    Download,
    Cancel,
}

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

/// "From Settings" plus the levels every pasted Source has.
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
