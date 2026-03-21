//! Sparsha Text - Font loading, text shaping, and glyph atlas using Parley.
//!
//! Stability: the supported 1.0 contract is the crate-root text-system API plus the `parley`
//! re-export for advanced configuration.

mod atlas;
mod system;

pub use atlas::GlyphAtlas;
pub use system::{ShapedText, TextStyle, TextSystem};

// Re-export parley for advanced font configuration
pub use parley;
