//! Shape rendering pass for rectangles with rounded corners.

use spark_core::{
    buffer::QuadBuffers,
    pipeline::{Pipeline, PipelineConfig},
    vertex::{ShapeInstance, Vertex2D},
    DynamicBuffer, GlobalUniforms, Rect,
};
use wgpu::{Device, Queue, RenderPass, TextureFormat};

/// WGSL shader for rendering shapes (rectangles with rounded corners and borders).
const SHAPE_SHADER: &str = r#"
struct Globals {
    viewport_size: vec2<f32>,
    scale_factor: f32,
    time: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct InstanceInput {
    @location(2) pos: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) corner_radius: f32,
    @location(6) border_width: f32,
    @location(7) border_color: vec4<f32>,
    @location(8) rotation: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) corner_radius: f32,
    @location(4) border_width: f32,
    @location(5) border_color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let local_pos = vertex.position * instance.size;
    let centered = local_pos - instance.size * 0.5;
    let s = sin(instance.rotation);
    let c = cos(instance.rotation);
    let rotated = vec2<f32>(
        centered.x * c - centered.y * s,
        centered.x * s + centered.y * c
    );
    let pixel_pos = instance.pos + instance.size * 0.5 + rotated;
    
    // Convert to clip space (-1 to 1)
    let clip_pos = (pixel_pos / globals.viewport_size) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(clip_pos.x, -clip_pos.y, 0.0, 1.0);
    
    out.color = instance.color;
    out.local_pos = local_pos;
    out.size = instance.size;
    out.corner_radius = instance.corner_radius;
    out.border_width = instance.border_width;
    out.border_color = instance.border_color;
    
    return out;
}

// Signed distance function for a rounded rectangle
fn sd_rounded_rect(pos: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let center_pos = pos - half_size;
    let q = abs(center_pos) - half_size + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = min(in.corner_radius, min(in.size.x, in.size.y) * 0.5);
    let dist = sd_rounded_rect(in.local_pos, in.size, radius);
    
    // Anti-aliasing
    let aa = 1.0;
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    
    if alpha < 0.001 {
        discard;
    }
    
    var final_color = in.color;
    
    // Border
    if in.border_width > 0.0 {
        let inner_dist = sd_rounded_rect(in.local_pos, in.size - in.border_width * 2.0, max(0.0, radius - in.border_width));
        let border_alpha = smoothstep(-aa, aa, inner_dist);
        final_color = mix(in.color, in.border_color, border_alpha);
    }
    
    return vec4<f32>(final_color.rgb, final_color.a * alpha);
}
"#;

/// Rendering pass for shapes (rectangles with rounded corners).
pub struct ShapePass {
    pipeline: Pipeline<GlobalUniforms>,
    quad_buffers: QuadBuffers,
    instance_buffer: DynamicBuffer<ShapeInstance>,
    instances: Vec<ShapeInstance>,
}

impl ShapePass {
    /// Create a new shape pass.
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        let pipeline = Pipeline::with_config(
            device,
            PipelineConfig {
                label: "shape_pipeline",
                shader_source: SHAPE_SHADER,
                vs_entry: "vs_main",
                fs_entry: "fs_main",
                target_format: format,
                vertex_layouts: &[Vertex2D::layout(), ShapeInstance::layout()],
                ..Default::default()
            },
        );

        let quad_buffers = QuadBuffers::new(device);
        let instance_buffer = DynamicBuffer::vertex(device, "shape_instances", 1024);

        Self {
            pipeline,
            quad_buffers,
            instance_buffer,
            instances: Vec::with_capacity(1024),
        }
    }

    /// Add a shape instance to be rendered.
    pub fn add_rect(
        &mut self,
        bounds: Rect,
        color: [f32; 4],
        corner_radius: f32,
        border_width: f32,
        border_color: [f32; 4],
    ) {
        self.add_rect_with_rotation(
            bounds,
            color,
            corner_radius,
            border_width,
            border_color,
            0.0,
        );
    }

    /// Add a shape instance with rotation.
    pub fn add_rect_with_rotation(
        &mut self,
        bounds: Rect,
        color: [f32; 4],
        corner_radius: f32,
        border_width: f32,
        border_color: [f32; 4],
        rotation: f32,
    ) {
        self.instances.push(ShapeInstance {
            pos: [bounds.x, bounds.y],
            size: [bounds.width, bounds.height],
            color,
            corner_radius,
            border_width,
            border_color,
            rotation,
            _padding: [0.0],
        });
    }

    /// Clear all pending instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Update GPU buffers with pending instances.
    pub fn prepare(&mut self, device: &Device, queue: &Queue, globals: &GlobalUniforms) {
        self.pipeline.update_uniforms(queue, globals);
        self.instance_buffer.write(device, queue, &self.instances);
    }

    /// Render all shapes to the given render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.instances.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_buffers.vertices.buffer().slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.buffer().slice(..));
        render_pass.set_index_buffer(
            self.quad_buffers.indices.buffer().slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.draw_indexed(0..6, 0, 0..self.instances.len() as u32);
    }

    /// Get the number of pending instances.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
