//! Text rendering pass using a glyph atlas.

use spark_core::{
    buffer::QuadBuffers,
    pipeline::{Pipeline, PipelineConfig},
    vertex::{GlyphInstance, Vertex2D},
    DynamicBuffer, GlobalUniforms,
};
use spark_text::GlyphAtlas;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Device, FilterMode, Queue, RenderPass,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, TextureFormat, TextureSampleType,
    TextureViewDimension,
};

/// WGSL shader for rendering text glyphs from an atlas.
const TEXT_SHADER: &str = r#"
struct Globals {
    viewport_size: vec2<f32>,
    scale_factor: f32,
    time: f32,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@group(1) @binding(0)
var atlas_texture: texture_2d<f32>;

@group(1) @binding(1)
var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct InstanceInput {
    @location(2) pos: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) uv_pos: vec2<f32>,
    @location(5) uv_size: vec2<f32>,
    @location(6) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform vertex position to pixel coordinates
    let pixel_pos = instance.pos + vertex.position * instance.size;
    
    // Convert to clip space (-1 to 1)
    let clip_pos = (pixel_pos / globals.viewport_size) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(clip_pos.x, -clip_pos.y, 0.0, 1.0);
    
    // Calculate UV from atlas coordinates
    out.uv = instance.uv_pos + vertex.uv * instance.uv_size;
    out.color = instance.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
    
    if alpha < 0.01 {
        discard;
    }
    
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

/// Rendering pass for text using glyph atlas.
pub struct TextPass {
    pipeline: Pipeline<GlobalUniforms>,
    atlas_bind_group_layout: BindGroupLayout,
    atlas_bind_group: Option<BindGroup>,
    sampler: Sampler,
    quad_buffers: QuadBuffers,
    instance_buffer: DynamicBuffer<GlyphInstance>,
    instances: Vec<GlyphInstance>,
}

impl TextPass {
    /// Create a new text pass.
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        // Create atlas bind group layout
        let atlas_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("text_atlas_bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline = Pipeline::with_config(
            device,
            PipelineConfig {
                label: "text_pipeline",
                shader_source: TEXT_SHADER,
                vs_entry: "vs_main",
                fs_entry: "fs_main",
                target_format: format,
                vertex_layouts: &[Vertex2D::layout(), GlyphInstance::layout()],
                extra_bind_group_layouts: &[&atlas_bind_group_layout],
                ..Default::default()
            },
        );

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("text_atlas_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let quad_buffers = QuadBuffers::new(device);
        let instance_buffer = DynamicBuffer::vertex(device, "text_instances", 4096);

        Self {
            pipeline,
            atlas_bind_group_layout,
            atlas_bind_group: None,
            sampler,
            quad_buffers,
            instance_buffer,
            instances: Vec::with_capacity(4096),
        }
    }

    /// Add glyph instances to be rendered.
    pub fn add_glyphs(&mut self, glyphs: &[GlyphInstance]) {
        self.instances.extend_from_slice(glyphs);
    }

    /// Clear all pending instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }

    /// Update GPU buffers with pending instances.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        globals: &GlobalUniforms,
        atlas: &GlyphAtlas,
    ) {
        self.pipeline.update_uniforms(queue, globals);
        self.instance_buffer.write(device, queue, &self.instances);

        // Recreate bind group if atlas changed
        self.atlas_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("text_atlas_bg"),
            layout: &self.atlas_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(atlas.view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
    }

    /// Render all text to the given render pass.
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.instances.is_empty() {
            return;
        }

        let Some(atlas_bind_group) = &self.atlas_bind_group else {
            return;
        };

        render_pass.set_pipeline(&self.pipeline.pipeline);
        render_pass.set_bind_group(0, &self.pipeline.bind_group, &[]);
        render_pass.set_bind_group(1, atlas_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_buffers.vertices.buffer().slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.buffer().slice(..));
        render_pass.set_index_buffer(
            self.quad_buffers.indices.buffer().slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.draw_indexed(0..6, 0, 0..self.instances.len() as u32);
    }

    /// Get the number of pending glyph instances.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
