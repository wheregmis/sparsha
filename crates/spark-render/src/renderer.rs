//! Main renderer that processes draw lists and issues GPU commands.

use crate::{DrawCommand, DrawList, ShapePass, TextPass};
use spark_core::{GlobalUniforms, Rect};
use spark_text::TextSystem;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};

/// The main renderer that processes draw lists and renders to the screen.
pub struct Renderer {
    shape_pass: ShapePass,
    text_pass: TextPass,
    globals: GlobalUniforms,
    clip_stack: Vec<Rect>,
    translation_stack: Vec<(f32, f32)>,
}

impl Renderer {
    /// Create a new renderer.
    pub fn new(device: &Device, format: TextureFormat) -> Self {
        Self {
            shape_pass: ShapePass::new(device, format),
            text_pass: TextPass::new(device, format),
            globals: GlobalUniforms::default(),
            clip_stack: Vec::new(),
            translation_stack: vec![(0.0, 0.0)],
        }
    }

    /// Update global uniforms (call once per frame before rendering).
    pub fn set_viewport(&mut self, width: f32, height: f32, scale_factor: f32) {
        self.globals.viewport_size = [width, height];
        self.globals.scale_factor = scale_factor;
    }

    /// Update time uniform.
    pub fn set_time(&mut self, time: f32) {
        self.globals.time = time;
    }

    /// Process a draw list and prepare GPU resources.
    pub fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        draw_list: &DrawList,
        text_system: &mut TextSystem,
    ) {
        self.shape_pass.clear();
        self.text_pass.clear();
        self.clip_stack.clear();
        self.translation_stack.clear();
        self.translation_stack.push((0.0, 0.0));

        for command in draw_list.commands() {
            match command {
                DrawCommand::Rect {
                    bounds,
                    color,
                    corner_radius,
                    border_width,
                    border_color,
                } => {
                    let translation = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let translated_bounds = Rect::new(
                        bounds.x + translation.0,
                        bounds.y + translation.1,
                        bounds.width,
                        bounds.height,
                    );
                    // Apply clipping if needed
                    let clipped_bounds = if let Some(clip) = self.clip_stack.last() {
                        match translated_bounds.intersection(clip) {
                            Some(b) => b,
                            None => continue, // Fully clipped, skip
                        }
                    } else {
                        translated_bounds
                    };

                    self.shape_pass.add_rect(
                        clipped_bounds,
                        color.to_array(),
                        *corner_radius,
                        *border_width,
                        border_color.to_array(),
                    );
                }
                DrawCommand::Text { glyphs } => {
                    // Apply clipping to glyphs
                    let translation = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));

                    if let Some(clip) = self.clip_stack.last() {
                        let mut visible_glyphs = Vec::with_capacity(glyphs.len());
                        for glyph in glyphs {
                            let mut translated_glyph = *glyph;
                            translated_glyph.pos[0] += translation.0;
                            translated_glyph.pos[1] += translation.1;

                            // Simple point-in-rect check for now
                            // Ideally we'd valid against glyph bounds, but point check is a good start
                            // to prevent massive overflow
                            if clip.contains(spark_core::Point::new(
                                translated_glyph.pos[0],
                                translated_glyph.pos[1],
                            )) {
                                visible_glyphs.push(translated_glyph);
                            }
                        }
                        if !visible_glyphs.is_empty() {
                            self.text_pass.add_glyphs(&visible_glyphs);
                        }
                    } else {
                        if translation == (0.0, 0.0) {
                            self.text_pass.add_glyphs(glyphs);
                        } else {
                            let mut translated = Vec::with_capacity(glyphs.len());
                            for glyph in glyphs {
                                let mut translated_glyph = *glyph;
                                translated_glyph.pos[0] += translation.0;
                                translated_glyph.pos[1] += translation.1;
                                translated.push(translated_glyph);
                            }
                            self.text_pass.add_glyphs(&translated);
                        }
                    }
                }
                DrawCommand::TextRun { run } => {
                    let translation = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let shaped = text_system.shape(device, queue, &run.text, &run.style, None);
                    if shaped.glyphs.is_empty() {
                        continue;
                    }

                    if let Some(clip) = self.clip_stack.last() {
                        let mut visible_glyphs = Vec::with_capacity(shaped.glyphs.len());
                        for glyph in &shaped.glyphs {
                            let mut translated_glyph = *glyph;
                            translated_glyph.pos[0] += run.position.0 + translation.0;
                            translated_glyph.pos[1] += run.position.1 + translation.1;
                            if clip.contains(spark_core::Point::new(
                                translated_glyph.pos[0],
                                translated_glyph.pos[1],
                            )) {
                                visible_glyphs.push(translated_glyph);
                            }
                        }
                        if !visible_glyphs.is_empty() {
                            self.text_pass.add_glyphs(&visible_glyphs);
                        }
                    } else {
                        let mut translated = Vec::with_capacity(shaped.glyphs.len());
                        for glyph in &shaped.glyphs {
                            let mut translated_glyph = *glyph;
                            translated_glyph.pos[0] += run.position.0 + translation.0;
                            translated_glyph.pos[1] += run.position.1 + translation.1;
                            translated.push(translated_glyph);
                        }
                        self.text_pass.add_glyphs(&translated);
                    }
                }
                DrawCommand::PushClip { bounds } => {
                    let translation = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    let translated_bounds = Rect::new(
                        bounds.x + translation.0,
                        bounds.y + translation.1,
                        bounds.width,
                        bounds.height,
                    );
                    // Intersect with current clip if any
                    let new_clip = if let Some(current) = self.clip_stack.last() {
                        translated_bounds
                            .intersection(current)
                            .unwrap_or(Rect::ZERO)
                    } else {
                        translated_bounds
                    };
                    self.clip_stack.push(new_clip);
                }
                DrawCommand::PopClip => {
                    self.clip_stack.pop();
                }
                DrawCommand::PushTranslation { offset } => {
                    let current = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
                    self.translation_stack
                        .push((current.0 + offset.0, current.1 + offset.1));
                }
                DrawCommand::PopTranslation => {
                    if self.translation_stack.len() > 1 {
                        self.translation_stack.pop();
                    }
                }
            }
        }

        // Update GPU buffers
        self.shape_pass.prepare(device, queue, &self.globals);
        if let Some(atlas) = text_system.atlas() {
            self.text_pass.prepare(device, queue, &self.globals, atlas);
        }
    }

    /// Render to the given texture view.
    pub fn render(
        &self,
        encoder: &mut CommandEncoder,
        target: &TextureView,
        clear_color: wgpu::Color,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("spark_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        // Render shapes first (background)
        self.shape_pass.render(&mut render_pass);

        // Render text on top
        self.text_pass.render(&mut render_pass);
    }

    /// Get the number of shape instances being rendered.
    pub fn shape_count(&self) -> usize {
        self.shape_pass.instance_count()
    }

    /// Get the number of glyph instances being rendered.
    pub fn glyph_count(&self) -> usize {
        self.text_pass.instance_count()
    }
}
