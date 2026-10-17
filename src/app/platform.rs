//! The small bridge to the browser. Everything here is a no-op on native.

/// True when the page can actually start a WebGPU swapchain.
pub fn webgpu_available() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let secure = js_sys::Reflect::get(&window, &"isSecureContext".into())
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let gpu = js_sys::Reflect::get(&window.navigator(), &"gpu".into())
            .ok()
            .map(|value| !value.is_undefined() && !value.is_null())
            .unwrap_or(false);
        secure && gpu
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        true
    }
}

/// Keep the canvas accessible: screen readers and automated checks can read
/// the current chapter even though the type is painted by egui.
pub fn set_page_label(number: &str, title: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Some(canvas) = document.get_element_by_id("atlas-canvas")
        {
            let _ = canvas.set_attribute("aria-label", &format!("深空图鉴 · {number} {title}"));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (number, title);
    }
}

pub fn set_motion(reduced: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Some(root) = document.document_element()
        {
            let _ = root.set_attribute("data-motion", if reduced { "reduced" } else { "full" });
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = reduced;
    }
}

pub fn mark_ready() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Some(root) = document.document_element()
        {
            let _ = root.class_list().add_1("atlas-ready");
        }
    }
}

pub fn mark_no_webgpu() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(document) = web_sys::window().and_then(|w| w.document())
            && let Some(root) = document.document_element()
        {
            let _ = root.class_list().add_1("no-webgpu");
            let _ = root.class_list().add_1("atlas-ready");
        }
    }
}

/// The OS-level request to reduce motion. Polled about once a second because
/// the user can change it while the atlas is open.
pub fn prefers_reduced_motion() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(window) = web_sys::window() else {
            return false;
        };
        match window.match_media("(prefers-reduced-motion: reduce)") {
            Ok(Some(query)) => query.matches(),
            Ok(None) => false,
            Err(error) => {
                log::debug!("matchMedia unavailable: {error:?}");
                false
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

pub fn hardware_concurrency() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .map(|window| window.navigator().hardware_concurrency() as u32)
            .unwrap_or(8)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(8)
    }
}

pub fn is_mobile_layout(width: f32, height: f32) -> bool {
    width < 760.0 || (width < 1000.0 && height > width * 1.12)
}
