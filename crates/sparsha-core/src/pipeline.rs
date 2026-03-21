//! Render pipeline abstractions.

use wgpu::*;

/// A GPU uniform buffer with typed data.
pub struct UniformBuffer<U: bytemuck::Pod + bytemuck::Zeroable> {
    pub buffer: Buffer,
    _phantom: std::marker::PhantomData<U>,
}

impl<U: bytemuck::Pod + bytemuck::Zeroable> UniformBuffer<U> {
    pub fn new(device: &Device) -> Self {
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("uniform_buffer"),
            size: std::mem::size_of::<U>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn write(&self, queue: &Queue, value: &U) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(value));
    }
}

/// Configuration for creating a render pipeline.
pub struct PipelineConfig<'a> {
    pub label: &'a str,
    pub shader_source: &'a str,
    pub vs_entry: &'a str,
    pub fs_entry: &'a str,
    pub target_format: TextureFormat,
    pub vertex_layouts: &'a [VertexBufferLayout<'a>],
    pub blend_state: Option<BlendState>,
    pub cull_mode: Option<Face>,
    pub extra_bind_group_layouts: &'a [&'a BindGroupLayout],
}

impl<'a> Default for PipelineConfig<'a> {
    fn default() -> Self {
        Self {
            label: "pipeline",
            shader_source: "",
            vs_entry: "vs_main",
            fs_entry: "fs_main",
            target_format: TextureFormat::Bgra8UnormSrgb,
            vertex_layouts: &[],
            blend_state: Some(BlendState::ALPHA_BLENDING),
            cull_mode: None, // No culling for 2D UI
            extra_bind_group_layouts: &[],
        }
    }
}

/// A typed render pipeline with uniforms.
pub struct Pipeline<U: bytemuck::Pod + bytemuck::Zeroable> {
    pub pipeline: RenderPipeline,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub uniform: UniformBuffer<U>,
}

impl<U: bytemuck::Pod + bytemuck::Zeroable> Pipeline<U> {
    /// Create a new pipeline with the given configuration.
    pub fn with_config(device: &Device, config: PipelineConfig) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(&format!("{}_shader", config.label)),
            source: ShaderSource::Wgsl(config.shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&format!("{}_uniform_bgl", config.label)),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Combine uniform bind group layout with any extra layouts
        let mut all_layouts: Vec<Option<&BindGroupLayout>> = vec![Some(&bind_group_layout)];
        all_layouts.extend(config.extra_bind_group_layouts.iter().copied().map(Some));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", config.label)),
            bind_group_layouts: &all_layouts,
            immediate_size: 0,
        });

        let uniform = UniformBuffer::<U>::new(device);

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("{}_uniform_bg", config.label)),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform.buffer.as_entire_binding(),
            }],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(config.label),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some(config.vs_entry),
                buffers: config.vertex_layouts,
                compilation_options: PipelineCompilationOptions::default(),
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: config.cull_mode,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some(config.fs_entry),
                targets: &[Some(ColorTargetState {
                    format: config.target_format,
                    blend: config.blend_state,
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            multiview_mask: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform,
        }
    }

    /// Legacy constructor for backwards compatibility.
    pub fn new(
        device: &Device,
        wgsl_src: &str,
        vs_entry: &str,
        fs_entry: &str,
        target_format: TextureFormat,
    ) -> Self {
        Self::with_config(
            device,
            PipelineConfig {
                label: "basic_pipeline",
                shader_source: wgsl_src,
                vs_entry,
                fs_entry,
                target_format,
                ..Default::default()
            },
        )
    }

    /// Update the uniform buffer with new data.
    pub fn update_uniforms(&mut self, queue: &Queue, value: &U) {
        self.uniform.write(queue, value);
    }
}
