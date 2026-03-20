//! Spark - A GPU-first cross-platform UI framework.
//!
//! # Example
//!
//! ```rust,no_run
//! use spark::prelude::*;
//!
//! fn main() {
//!     // On web, call init_web() first
//!     #[cfg(target_arch = "wasm32")]
//!     spark::init_web();
//!     
//!     App::new()
//!         .with_title("My App")
//!         .run(|| {
//!             Box::new(Container::new()
//!                 .child(Button::new("Click me!")))
//!         });
//! }
//! ```

pub mod accessibility;
mod app;

#[cfg(target_arch = "wasm32")]
mod dom_renderer;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod web_app;

pub use app::{App, AppConfig};

#[cfg(target_arch = "wasm32")]
pub use web::init_web;

/// Re-exports of commonly used types.
pub mod prelude {
    pub use crate::{App, AppConfig};
    pub use spark_core::{Color, Rect};
    pub use spark_input::{InputEvent, Key, Modifiers, PointerButton};
    pub use spark_layout::taffy;
    pub use spark_widgets::{
        Button, ButtonStyle, Checkbox, CheckboxStyle, Container, EventResponse, List,
        ListDirection, Scroll, ScrollDirection, Text, TextAlign, TextInput, Widget,
    };
}

// Re-export sub-crates
pub use spark_core as core;
pub use spark_input as input;
pub use spark_layout as layout;
pub use spark_render as render;
pub use spark_text as text;
pub use spark_widgets as widgets;
