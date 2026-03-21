//! Web platform support for WASM/WebGPU.

/// Initialize the web platform (call this before App::run on web).
#[cfg(target_arch = "wasm32")]
pub fn init_web() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Debug).expect("error initializing logger");
    log::info!("Sparsh web platform initialized");
}

/// Macro to set up web entry point.
#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! wasm_main {
    ($main_fn:ident) => {
        #[wasm_bindgen::prelude::wasm_bindgen(start)]
        pub fn wasm_start() {
            $crate::init_web();
            $main_fn();
        }
    };
}
