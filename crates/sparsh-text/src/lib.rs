//! Sparsh Text - Font loading, text shaping, and glyph atlas using Parley.

mod atlas;
mod system;

pub use atlas::GlyphAtlas;
pub use system::{ShapedText, TextStyle, TextSystem};

// Re-export parley for advanced font configuration
pub use parley;
