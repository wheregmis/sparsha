//! Sparsha Input - Event handling, focus management, and hit testing.
//!
//! Uses [ui-events](https://github.com/endoli/ui-events) from the Linebender
//! ecosystem for W3C-compliant UI event types.
//!
//! Stability: the supported 1.0 contract is the crate-root action/event/focus APIs plus the
//! `ui_events` re-export. Platform glue stays internal until after 1.0.

mod action;
mod events;
mod focus;
mod hit_test;

// Re-export ui-events types
pub use ui_events;

// Action system
pub use action::{
    Action, ActionContext, ActionHandler, ActionMapper, CustomAction, StandardAction,
};

// Our wrapper types
pub use events::{
    shortcuts, CompositionEvent, InputEvent, Key, KeyState, KeyboardEvent, Modifiers, NamedKey,
    PointerButton, PointerId, PointerState, PointerType, ScrollDelta,
};
pub use focus::FocusManager;
pub use hit_test::{hit_test, hit_test_all, hit_test_filtered, HitTestResult};
