//! Vertex types for GPU rendering.

use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, VertexStepMode};

/// A basic 2D vertex with position and UV coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex2D {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

impl Vertex2D {
    pub const ATTRIBS: [VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x2,  // uv
    ];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    /// Unit quad vertices (0,0) to (1,1) - for instanced rendering.
    pub const UNIT_QUAD: [Self; 4] = [
        Self {
            position: [0.0, 0.0],
            uv: [0.0, 0.0],
        },
        Self {
            position: [1.0, 0.0],
            uv: [1.0, 0.0],
        },
        Self {
            position: [1.0, 1.0],
            uv: [1.0, 1.0],
        },
        Self {
            position: [0.0, 1.0],
            uv: [0.0, 1.0],
        },
    ];

    /// Indices for a unit quad (two triangles).
    pub const UNIT_QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
}

/// Instance data for rendering a shape (rectangle with optional rounded corners).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ShapeInstance {
    /// Position in pixels (top-left corner).
    pub pos: [f32; 2],
    /// Size in pixels (width, height).
    pub size: [f32; 2],
    /// RGBA color (0.0 - 1.0).
    pub color: [f32; 4],
    /// Corner radius in pixels.
    pub corner_radius: f32,
    /// Border width in pixels.
    pub border_width: f32,
    /// Border color RGBA.
    pub border_color: [f32; 4],
    /// Rotation in radians around the instance center.
    pub rotation: f32,
    /// Padding for alignment.
    pub _padding: [f32; 1],
}

impl Default for ShapeInstance {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0],
            size: [100.0, 100.0],
            color: [1.0, 1.0, 1.0, 1.0],
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: [0.0, 0.0, 0.0, 0.0],
            rotation: 0.0,
            _padding: [0.0],
        }
    }
}

impl ShapeInstance {
    pub const ATTRIBS: [VertexAttribute; 7] = wgpu::vertex_attr_array![
        // Start at location 2 (after Vertex2D uses 0 and 1)
        2 => Float32x2,   // pos
        3 => Float32x2,   // size
        4 => Float32x4,   // color
        5 => Float32,     // corner_radius
        6 => Float32,     // border_width
        7 => Float32x4,   // border_color
        8 => Float32,     // rotation
        // _padding not needed in shader
    ];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Instance data for rendering a text glyph.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GlyphInstance {
    /// Position in pixels (top-left corner).
    pub pos: [f32; 2],
    /// Size in pixels (width, height).
    pub size: [f32; 2],
    /// UV coordinates in atlas (top-left).
    pub uv_pos: [f32; 2],
    /// UV size in atlas.
    pub uv_size: [f32; 2],
    /// Text color RGBA.
    pub color: [f32; 4],
}

impl GlyphInstance {
    pub const ATTRIBS: [VertexAttribute; 5] = wgpu::vertex_attr_array![
        2 => Float32x2,   // pos
        3 => Float32x2,   // size
        4 => Float32x2,   // uv_pos
        5 => Float32x2,   // uv_size
        6 => Float32x4,   // color
    ];

    pub fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}
