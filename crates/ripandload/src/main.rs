// On Windows, don't open a console window next to the GUI in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod settings_tab;

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ripandload=info".into()),
        )
        .init();
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

/// OpenGL (glow) by default: wgpu flickers and lags when resizing on Windows.
/// Set `RIPANDLOAD_RENDERER=wgpu` to try the wgpu renderer instead.
fn renderer_from_env() -> eframe::Renderer {
    match std::env::var("RIPANDLOAD_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    }
}
