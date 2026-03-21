//! Web platform support for WASM/WebGPU.

use crate::AppRunError;

/// Initialize the web platform (call this before App::run on web).
#[cfg(target_arch = "wasm32")]
pub fn init_web() -> Result<(), AppRunError> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    if let Err(err) = console_log::init_with_level(log::Level::Debug) {
        web_sys::console::warn_1(
            &format!("sparsh logger already initialized or unavailable: {err}").into(),
        );
    }
    log::info!("Sparsh web platform initialized");
    Ok(())
}

/// Macro to set up web entry point.
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! wasm_main {
    ($main_fn:ident) => {
        #[wasm_bindgen::prelude::wasm_bindgen(start)]
        pub fn wasm_start() -> Result<(), wasm_bindgen::JsValue> {
            $crate::init_web().map_err(|err| {
                let message = format!("failed to initialize Sparsh web runtime: {err}");
                web_sys::console::error_1(&message.clone().into());
                wasm_bindgen::JsValue::from_str(&message)
            })?;

            $main_fn().map_err(|err| {
                let message = format!("failed to start Sparsh app: {err}");
                web_sys::console::error_1(&message.clone().into());
                wasm_bindgen::JsValue::from_str(&message)
            })
        }
    };
}
