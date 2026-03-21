//! Sparsha Widgets - UI widget library.
//!
//! Stability: the supported 1.0 contract is the crate-root widget/theme/context re-export set.

mod accessibility;
mod button;
mod checkbox;
mod container;
#[doc(hidden)]
pub mod context;
mod control_state;
mod draw_surface;
mod for_each;
mod into_widget;
mod list;
mod scroll;
mod scroll_model;
mod semantics;
mod text;
mod text_area;
mod text_editor;
mod text_input;
mod theme;
mod widget;

pub use accessibility::{AccessibilityAction, AccessibilityInfo, AccessibilityRole};
pub use button::{Button, ButtonState, ButtonStyle};
pub use checkbox::{Checkbox, CheckboxStyle};
pub use container::Container;
pub use context::{
    BuildContext, EventCommands, EventContext, LayoutContext, PaintCommands, PaintContext,
};
pub use draw_surface::{DrawSurface, DrawSurfaceContext};
pub use for_each::ForEach;
pub use into_widget::IntoWidget;
pub use list::{List, ListDirection};
pub use scroll::{Scroll, ScrollDirection, ScrollbarStyle};
pub use semantics::Semantics;
pub use text::{Text, TextAlign};
pub use text_area::{TextArea, TextAreaStyle};
pub use text_editor::TextEditorState;
pub use text_input::{TextInput, TextInputStyle};
pub use theme::{
    current_theme, set_current_theme, Theme, ThemeColors, ThemeControls, ThemeRadii, ThemeSpacing,
    ThemeTypography,
};
pub use widget::Widget;

// Re-export layout types for convenience
pub use sparsha_layout::{styles, taffy, WidgetId};

#[cfg(test)]
mod test_helpers;
