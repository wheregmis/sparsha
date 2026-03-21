//! Sparsh Core - GPU primitives, pipelines, and low-level rendering.
//!
//! Stability: the supported 1.0 contract is the crate-root re-export set below.
//! Internal implementation modules are intentionally private.

mod buffer;
mod pipeline;
mod types;
mod vertex;
mod wgpu_init;

// Re-exports
pub use buffer::{DynamicBuffer, QuadBuffers, StaticBuffer};
pub use pipeline::{Pipeline, PipelineConfig, UniformBuffer};
pub use types::{Color, GlobalUniforms, Point, Rect};
pub use vertex::{GlyphInstance, ShapeInstance, Vertex2D};
pub use wgpu_init::{init_wgpu, SurfaceState, WgpuInitError};

#[cfg(not(target_arch = "wasm32"))]
pub use wgpu_init::init_wgpu_headless;

// Re-export wgpu and glam for convenience
pub use glam;
pub use wgpu;
