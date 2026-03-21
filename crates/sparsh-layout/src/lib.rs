//! Sparsh Layout - Flexbox layout engine via taffy.
//!
//! Stability: the supported 1.0 contract is the crate-root `LayoutTree`, `ComputedLayout`,
//! `WidgetId`, `styles`, and `taffy` re-exports.

mod tree;

pub use tree::{styles, ComputedLayout, LayoutTree, WidgetId};

// Re-export taffy for style definitions
pub use taffy;
