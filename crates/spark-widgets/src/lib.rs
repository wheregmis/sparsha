//! Spark Widgets - UI widget library.

mod button;
mod checkbox;
mod container;
mod context;
mod list;
mod scroll;
mod text;
mod text_input;
mod widget;

pub use button::{Button, ButtonState, ButtonStyle};
pub use checkbox::{Checkbox, CheckboxStyle};
pub use container::Container;
pub use context::{EventContext, LayoutContext, PaintContext};
pub use list::{List, ListDirection};
pub use scroll::{Scroll, ScrollDirection, ScrollbarStyle};
pub use text::{Text, TextAlign};
pub use text_input::{TextInput, TextInputStyle};
pub use widget::{EventResponse, Widget};

// Re-export layout types for convenience
pub use spark_layout::{styles, taffy, WidgetId};

#[cfg(test)]
mod test_helpers;
