//! Sparsha - A GPU-first cross-platform UI framework.
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
//! use sparsha::prelude::*;
//!
//! fn main() -> Result<(), sparsha::AppRunError> {
//!     // On web, call init_web() first
//!     #[cfg(target_arch = "wasm32")]
//!     sparsha::init_web()?;
//!
//!     App::new()
//!         .title("My App")
//!         .theme(Theme::light())
//!         .router(
//!             Router::new()
//!                 .route("/", || Container::new().child(Button::new("Click me!")))
//!                 .fallback("/"),
//!         )
//!         .run()
//! }
//! ```
//!
//! # Hybrid Rendering On Web
//!
//! Sparsha keeps retained DOM rendering as the default on web.
//! For scenes that are fundamentally draw-heavy, use [`widgets::DrawSurface`] to embed a
//! GPU-rendered surface inside the DOM while continuing to paint normal overlays through the
//! widget tree.
//!
//! # Task Runtime
//!
//! [`TaskRuntime`] is part of the supported crate-root 1.0 surface.
//! The built-in executor currently supports the task kinds `echo`, `sleep_echo`, and
//! `analyze_text`, along with the existing `spawn`, `spawn_keyed`, `cancel`, and result-delivery
//! semantics on native and web. Custom task registration is not part of the 1.0 contract.

mod accessibility;
mod app;
mod component;
mod platform;
mod router;
mod runtime_core;
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
pub use component::{component, component_builder, Component, ComponentContext, TaskHook};
pub use router::{hash_to_path, path_to_hash, Navigator, Route, Router, RouterTransition};
pub use sparsha_widgets::{
    current_theme, current_viewport, lerp_color, AccessibilityAction, AccessibilityInfo,
    AccessibilityRole, AnimationEasing, ForEach, ImplicitAnimation, IntoWidget, Semantics,
    TextArea, TextAreaStyle, TextEditorState, TextInput, TextInputStyle, Theme, ThemeColors,
    ThemeControls, ThemeRadii, ThemeSpacing, ThemeTypography, Tween, ViewportClass, ViewportInfo,
    ViewportOrientation, WidgetChildMode,
};
pub use tasks::{
    Generation, TaskHandle, TaskId, TaskKey, TaskPayload, TaskPolicy, TaskResult,
    TaskResultSubscription, TaskRuntime, TaskRuntimeInitError, TaskStatus,
};

#[cfg(target_arch = "wasm32")]
pub use web::init_web;

/// Re-exports of commonly used types.
pub mod prelude {
    pub use crate::tasks::{
        Generation, TaskHandle, TaskId, TaskKey, TaskPayload, TaskPolicy, TaskResult,
        TaskResultSubscription, TaskRuntime, TaskStatus,
    };
    pub use crate::{
        component, component_builder, App, AppRunError, Component, ComponentContext, Navigator,
        Route, Router, RouterTransition, TaskHook, ThemeInput, ThemeMode, ThemeModeInput,
    };
    pub use sparsha_core::{Color, Rect};
    pub use sparsha_input::{InputEvent, Key, Modifiers, PointerButton};
    pub use sparsha_layout::taffy;
    pub use sparsha_signals::{Effect, Memo, ReadSignal, Signal, WriteSignal};
    pub use sparsha_widgets::{
        current_theme, current_viewport, lerp_color, AccessibilityAction, AccessibilityInfo,
        AccessibilityRole, AnimationEasing, BuildContext, Button, ButtonStyle, Checkbox,
        CheckboxStyle, Container, DrawSurface, EventCommands, ForEach, ImplicitAnimation,
        IntoWidget, List, ListDirection, Scroll, ScrollDirection, Semantics, Text, TextAlign,
        TextArea, TextAreaStyle, TextEditorState, TextInput, TextInputStyle, Theme, ThemeColors,
        ThemeControls, ThemeRadii, ThemeSpacing, ThemeTypography, Tween, ViewportClass,
        ViewportInfo, ViewportOrientation, Widget, WidgetChildMode,
    };
}

// Re-export sub-crates
pub use sparsha_core as core;
pub use sparsha_input as input;
pub use sparsha_layout as layout;
pub use sparsha_render as render;
pub use sparsha_signals as signals;
pub use sparsha_text as text;
pub use sparsha_widgets as widgets;
