//! Sparsh - A GPU-first cross-platform UI framework.
//!
//! # Stability
//!
//! The supported 1.0 API surface is the set of crate-root re-exports documented in this crate
//! and in the repository docs. Internal modules and implementation details are intentionally kept
//! out of the semver contract until after 1.0.
//!
//! # Example
//!
//! ```rust,no_run
//! use sparsh::prelude::*;
//!
//! fn main() -> Result<(), sparsh::AppRunError> {
//!     // On web, call init_web() first
//!     #[cfg(target_arch = "wasm32")]
//!     sparsh::init_web()?;
//!
//!     App::new()
//!         .title("My App")
//!         .theme(Theme::light())
//!         .router(
//!             Router::new()
//!                 .route("/", || {
//!                     Box::new(Container::new().child(Button::new("Click me!")))
//!                 })
//!                 .fallback("/"),
//!         )
//!         .run()
//! }
//! ```
//!
//! # Hybrid Rendering On Web
//!
//! Sparsh keeps retained DOM rendering as the default on web.
//! For scenes that are fundamentally draw-heavy, use [`widgets::DrawSurface`] to embed a
//! GPU-rendered surface inside the DOM while continuing to paint normal overlays through the
//! widget tree.

mod accessibility;
mod app;
mod router;
mod runtime_widget;
mod tasks;

#[cfg(target_arch = "wasm32")]
mod dom_renderer;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod web_app;
#[cfg(target_arch = "wasm32")]
mod web_surface_manager;
#[cfg(target_arch = "wasm32")]
mod web_text_metrics;

pub use app::{App, AppRunError, ThemeInput, ThemeMode, ThemeModeInput};
pub use router::{hash_to_path, path_to_hash, Navigator, Route, Router};
pub use sparsh_widgets::{
    TextArea, TextAreaStyle, TextEditorState, TextInput, TextInputStyle, Theme, ThemeColors,
    ThemeRadii, ThemeSpacing, ThemeTypography,
};
pub use tasks::{
    Generation, TaskHandle, TaskId, TaskKey, TaskPayload, TaskPolicy, TaskResult, TaskRuntime,
    TaskRuntimeInitError, TaskStatus,
};

#[cfg(target_arch = "wasm32")]
pub use web::init_web;

/// Re-exports of commonly used types.
pub mod prelude {
    pub use crate::tasks::{
        Generation, TaskHandle, TaskId, TaskKey, TaskPayload, TaskPolicy, TaskResult, TaskRuntime,
        TaskStatus,
    };
    pub use crate::{
        App, AppRunError, Navigator, Route, Router, ThemeInput, ThemeMode, ThemeModeInput,
    };
    pub use sparsh_core::{Color, Rect};
    pub use sparsh_input::{InputEvent, Key, Modifiers, PointerButton};
    pub use sparsh_layout::taffy;
    pub use sparsh_signals::{Effect, Memo, ReadSignal, Signal, WriteSignal};
    pub use sparsh_widgets::{
        BuildContext, Button, ButtonStyle, Checkbox, CheckboxStyle, Container, DrawSurface,
        EventCommands, List, ListDirection, Scroll, ScrollDirection, Text, TextAlign, TextArea,
        TextAreaStyle, TextEditorState, TextInput, TextInputStyle, Theme, ThemeColors, ThemeRadii,
        ThemeSpacing, ThemeTypography, Widget,
    };
}

// Re-export sub-crates
pub use sparsh_core as core;
pub use sparsh_input as input;
pub use sparsh_layout as layout;
pub use sparsh_render as render;
pub use sparsh_signals as signals;
pub use sparsh_text as text;
pub use sparsh_widgets as widgets;
