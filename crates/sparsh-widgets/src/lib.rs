//! Sparsh Widgets - UI widget library.

mod button;
mod checkbox;
mod container;
mod context;
mod draw_surface;
mod list;
mod scroll;
mod text;
mod text_input;
mod widget;

pub use button::{Button, ButtonState, ButtonStyle};
pub use checkbox::{Checkbox, CheckboxStyle};
pub use container::Container;
pub use context::{
    BuildContext, EventCommands, EventContext, LayoutContext, PaintCommands, PaintContext,
};
pub use draw_surface::{DrawSurface, DrawSurfaceContext};
pub use list::{List, ListDirection};
pub use scroll::{Scroll, ScrollDirection, ScrollbarStyle};
pub use text::{Text, TextAlign};
pub use text_input::{TextInput, TextInputStyle};
pub use widget::Widget;

// Re-export layout types for convenience
pub use sparsh_layout::{styles, taffy, WidgetId};

#[cfg(test)]
mod test_helpers;
