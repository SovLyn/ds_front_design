//! Entry point. Native uses a normal eframe window; the browser waits for a
//! WebGPU-capable canvas and only then starts the same application.

/// Ask for the discrete GPU when the platform honours the WebGPU hint.
///
/// This is only a hint. Chrome on Windows currently ignores `powerPreference`
/// (Chromium bug 369219127), so the OS/browser selects the adapter there; the
/// README explains how to force Chrome onto the high-performance GPU.
fn wgpu_configuration() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};

    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.power_preference = eframe::wgpu::PowerPreference::HighPerformance;

    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use eframe::egui;

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: wgpu_configuration(),
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([360.0, 600.0])
            .with_title("Deep Space Atlas · 深空图鉴"),
        ..Default::default()
    };

    eframe::run_native(
        "deep-space-atlas",
        options,
        Box::new(|cc| Ok(Box::new(deep_space_atlas::app::AtlasApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use deep_space_atlas::app::platform;
    use wasm_bindgen::JsCast;

    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    wasm_bindgen_futures::spawn_local(async move {
        if !platform::webgpu_available() {
            log::warn!("WebGPU is unavailable; showing the static fallback page");
            platform::mark_no_webgpu();
            return;
        }

        let document = match web_sys::window().and_then(|window| window.document()) {
            Some(document) => document,
            None => {
                log::error!("no document");
                platform::mark_no_webgpu();
                return;
            }
        };

        let canvas = match document
            .get_element_by_id("atlas-canvas")
            .and_then(|element| element.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        {
            Some(canvas) => canvas,
            None => {
                log::error!("#atlas-canvas is missing");
                platform::mark_no_webgpu();
                return;
            }
        };

        let options = eframe::WebOptions {
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: wgpu_configuration(),
            ..Default::default()
        };
        let runner = eframe::WebRunner::new();
        let result = runner
            .start(
                canvas,
                options,
                Box::new(|cc| Ok(Box::new(deep_space_atlas::app::AtlasApp::new(cc)))),
            )
            .await;

        if let Err(error) = result {
            log::error!("failed to start the atlas: {error:?}");
            platform::mark_no_webgpu();
        }

        // The runner owns the animation loop for the lifetime of the page.
        std::mem::forget(runner);
    });
}
