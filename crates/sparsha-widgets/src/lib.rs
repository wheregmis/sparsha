//! Sparsha Widgets - UI widget library.
//!
//! Stability: the supported 1.0 contract is the crate-root widget/theme/context re-export set.

mod accessibility;
mod animation;
mod app_shell;
mod button;
mod checkbox;
mod container;
#[doc(hidden)]
pub mod context;
mod control_state;
mod draw_surface;
mod for_each;
mod into_widget;
mod layout_helpers;
mod list;
mod provider;
mod scroll;
mod scroll_model;
mod semantics;
mod text;
mod text_area;
mod text_editor;
mod text_editor_widget;
mod text_input;
mod theme;
mod viewport;
mod widget;

pub use accessibility::{AccessibilityAction, AccessibilityInfo, AccessibilityRole};
pub use animation::{lerp_color, AnimationEasing, ImplicitAnimation, Tween};
pub use app_shell::{AppBar, FloatingActionButton, Scaffold};
pub use button::{Button, ButtonState, ButtonStyle};
pub use checkbox::{Checkbox, CheckboxStyle};
pub use container::{Container, CrossAxisAlignment, MainAxisAlignment};
pub use context::{
    BuildContext, EventCommands, EventContext, LayoutContext, PaintCommands, PaintContext,
};
pub use draw_surface::{DrawSurface, DrawSurfaceContext};
pub use for_each::ForEach;
pub use into_widget::IntoWidget;
pub use layout_helpers::{
    Align, Alignment, Center, Expanded, Padding, Positioned, SizedBox, Spacer, Stack,
};
pub use list::{List, ListDirection};
pub use provider::Provider;
pub use scroll::{Scroll, ScrollDirection, ScrollbarStyle};
pub use semantics::Semantics;
pub use sparsha_text::{TextBreakMode, TextWrap};
pub use text::{Text, TextAlign, TextOverflow, TextVariant};
pub use text_area::{TextArea, TextAreaStyle};
pub use text_editor::TextEditorState;
pub use text_input::{TextInput, TextInputStyle};
pub use theme::{
    current_theme, set_current_theme, Theme, ThemeColors, ThemeControls, ThemeRadii, ThemeSpacing,
    ThemeTypography,
};
#[doc(hidden)]
pub use viewport::set_current_viewport;
pub use viewport::{current_viewport, ViewportClass, ViewportInfo, ViewportOrientation};
pub(crate) use viewport::{
    responsive_text_area_min_height, responsive_theme_controls, responsive_typography,
};
pub use widget::{Widget, WidgetChildMode};

// Re-export layout types for convenience
pub use sparsha_layout::{styles, taffy, WidgetId};

#[cfg(test)]
mod test_helpers;
