//! Spark Widgets - UI widget library.

mod button;
mod container;
mod context;
mod scroll;
mod text;
mod text_input;
mod widget;

pub use button::{Button, ButtonState, ButtonStyle};
pub use container::Container;
pub use context::{EventContext, LayoutContext, PaintContext};
pub use scroll::{Scroll, ScrollDirection, ScrollbarStyle};
pub use text::{Text, TextAlign};
pub use text_input::{TextInput, TextInputStyle};
pub use widget::{EventResponse, Widget};

// Re-export layout types for convenience
pub use spark_layout::{styles, taffy, WidgetId};

#[cfg(test)]
mod test_helpers;
