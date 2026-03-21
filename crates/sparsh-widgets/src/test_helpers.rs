//! Test helpers for widget event and paint tests.
//! Only compiled when `cfg(test)`.

use super::context::EventContext;
use sparsh_core::Rect;
use sparsh_input::{FocusManager, InputEvent, PointerButton};
use sparsh_layout::{ComputedLayout, LayoutTree, WidgetId};

/// Build an `EventContext` for testing. Caller must keep `layout_tree` and `focus` alive.
pub fn mock_event_context<'a>(
    layout: ComputedLayout,
    layout_tree: &'a LayoutTree,
    focus: &'a mut FocusManager,
    widget_id: WidgetId,
    has_capture: bool,
) -> EventContext<'a> {
    EventContext {
        layout,
        layout_tree,
        focus,
        widget_id,
        has_capture,
        commands: Default::default(),
    }
}

/// Create a `ComputedLayout` with the given bounds (e.g. for a 100x40 button at origin).
pub fn layout_bounds(x: f32, y: f32, width: f32, height: f32) -> ComputedLayout {
    ComputedLayout::new(Rect::new(x, y, width, height))
}

/// `InputEvent::PointerMove` at the given position.
pub fn pointer_move_at(x: f32, y: f32) -> InputEvent {
    InputEvent::PointerMove {
        pos: glam::Vec2::new(x, y),
    }
}

/// `InputEvent::PointerDown` (primary button) at the given position.
pub fn pointer_down_at(x: f32, y: f32) -> InputEvent {
    InputEvent::PointerDown {
        pos: glam::Vec2::new(x, y),
        button: PointerButton::Primary,
    }
}

/// `InputEvent::PointerUp` (primary button) at the given position.
pub fn pointer_up_at(x: f32, y: f32) -> InputEvent {
    InputEvent::PointerUp {
        pos: glam::Vec2::new(x, y),
        button: PointerButton::Primary,
    }
}
