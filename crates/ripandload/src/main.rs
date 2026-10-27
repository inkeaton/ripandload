//! The ripandload desktop app.
//!
//! ripandload downloads music from a Source with streamrip into the Staging
//! folder, then Uploads that folder to a friend group's Navidrome Server.
//! This binary is its GUI: one eframe window with a Download, an Upload and
//! a Settings tab.
//!
//! This file only starts the program: it sets up logging, picks the
//! renderer and opens the window. What the window shows and does is in
//! `app` and the tab modules. Running the tools, Settings and the Streamrip
//! config live in `ripandload_core`.
//!
//! # Environment
//!
//! - `RUST_LOG`: the log filter, `ripandload=info` when unset.
//! - `RIPANDLOAD_RENDERER`: `wgpu` to draw with wgpu instead of OpenGL.

// A release build on Windows opens no console window next to the GUI.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod download_tab;
mod path_picker;
mod settings_tab;
mod upload_tab;
mod wording;

use std::sync::Mutex;

use ripandload_core::log_file;

/// Starts logging, then runs the app's window until the user closes it.
///
/// # Errors
///
/// The error from [`eframe::run_native`] if the window or its renderer
/// can't be created.
fn main() -> eframe::Result {
    init_logging();
    tracing::info!(version = ripandload_core::VERSION, "starting ripandload");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ripandload")
            .with_inner_size([800.0, 600.0]),
        renderer: renderer_from_env(),
        ..Default::default()
    };

    eframe::run_native(
        "ripandload",
        options,
        Box::new(|_cc| Ok(Box::new(app::RipAndLoadApp::new()))),
    )
}

/// Sends `tracing` output to stderr and to the log file, panics included.
///
/// The log file is [`log_file::LOG`] in [`log_file::default_dir`]. It's the
/// only place a release build on Windows can log to, since it has no
/// console, and that's why panics are logged too. Without a data folder, or
/// when the file can't be opened, logging goes to stderr only, and a warning
/// logged right after says why.
///
/// # Panics
///
/// Panics if a global `tracing` subscriber is already set, which only
/// happens if this is called twice.
fn init_logging() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "ripandload=info".into());
    let dir = log_file::default_dir();
    let (file, open_error) = match dir.as_deref().map(log_file::open).transpose() {
        Ok(file) => (file, None),
        Err(err) => (None, Some(err)),
    };
    let file_layer = file.map(|file| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
    });
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer)
        .init();

    match (dir, open_error) {
        (Some(dir), Some(err)) => {
            tracing::warn!(dir = %dir.display(), error = %err, "couldn't open the log file");
        }
        (Some(dir), None) => {
            tracing::info!(log = %dir.join(log_file::LOG).display(), "logging to file")
        }
        (None, _) => tracing::warn!("no data folder found; logging to stderr only"),
    }

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("{info}");
        previous_hook(info);
    }));
}

/// Returns wgpu if `RIPANDLOAD_RENDERER` is `wgpu`, else OpenGL (glow).
///
/// OpenGL is the default because wgpu flickers and lags when the window is
/// resized on Windows.
fn renderer_from_env() -> eframe::Renderer {
    match std::env::var("RIPANDLOAD_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    }
}
