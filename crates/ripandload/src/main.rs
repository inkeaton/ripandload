// On Windows, don't open a console window next to the GUI in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod download_tab;
mod settings_tab;
mod upload_tab;

use std::sync::Mutex;

use ripandload_core::log_file;

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

/// Logs to stderr and to `ripandload.log` in the platform data dir, the only
/// place a release build on Windows can log to, since it has no console.
/// Panics are logged too, for the same reason.
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

/// OpenGL (glow) by default: wgpu flickers and lags when resizing on Windows.
/// Set `RIPANDLOAD_RENDERER=wgpu` to try the wgpu renderer instead.
fn renderer_from_env() -> eframe::Renderer {
    match std::env::var("RIPANDLOAD_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    }
}
