//! Sparsh Render - Draw list, batching, and GPU rendering passes.
//!
//! Stability: the supported 1.0 contract is the crate-root draw list and renderer re-export set.

mod commands;
mod renderer;
mod shape_pass;
mod text_pass;

pub use commands::{DrawCommand, DrawList, TextRun};
pub use renderer::Renderer;
pub use shape_pass::ShapePass;
pub use text_pass::TextPass;
