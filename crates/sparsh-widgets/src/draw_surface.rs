//! Draw-surface widget for draw-heavy scenes.
//!
//! `DrawSurface` is the opt-in bridge between Sparsh's normal retained widget tree and a
//! scene-style draw callback.
//!
//! On native targets the scene is rendered through the shared GPU renderer.
//! On web targets Sparsh can embed the scene inside a dedicated `<canvas>` while still allowing
//! the widget to paint normal DOM-backed overlays through the regular `paint()` path.

use crate::{PaintCommands, PaintContext, Widget};
use sparsh_core::{Color, Point, Rect};
use sparsh_layout::{taffy, WidgetId};
use sparsh_render::DrawList;
use sparsh_text::TextStyle;

type SceneCallback = Box<dyn Fn(&mut DrawSurfaceContext)>;

pub struct DrawSurface {
    id: WidgetId,
    style: taffy::Style,
    scene: SceneCallback,
}

/// Paint context for a `DrawSurface` scene callback.
///
/// The scene callback should use this for draw-heavy, animation-heavy content. Regular widget
/// overlays such as labels, controls, or HUD panels should usually stay in the widget's normal
/// `paint()` path so they continue to participate in the default retained renderer.
pub struct DrawSurfaceContext<'a> {
    pub draw_list: &'a mut DrawList,
    pub bounds: Rect,
    pub scale_factor: f32,
    pub elapsed_time: f32,
    pub commands: &'a mut PaintCommands,
}

impl<'a> DrawSurfaceContext<'a> {
    pub fn fill_rect(&mut self, bounds: Rect, color: Color) {
        self.draw_list.rect(bounds, color);
    }

    pub fn fill_rounded_rect(&mut self, bounds: Rect, color: Color, radius: f32) {
        self.draw_list
            .rounded_rect(bounds, color, radius * self.scale_factor);
    }

    pub fn fill_bordered_rect(
        &mut self,
        bounds: Rect,
        color: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        self.draw_list.bordered_rect(
            bounds,
            color,
            radius * self.scale_factor,
            border_width * self.scale_factor,
            border_color,
        );
    }

    pub fn stroke_line(&mut self, start: Point, end: Point, thickness: f32, color: Color) {
        self.draw_list.line(
            (start.x, start.y),
            (end.x, end.y),
            thickness * self.scale_factor,
            color,
        );
    }

    pub fn draw_text(&mut self, text: impl Into<String>, style: &TextStyle, x: f32, y: f32) {
        self.draw_list.text_run(
            text,
            TextStyle {
                font_size: style.font_size * self.scale_factor,
                ..style.clone()
            },
            x,
            y,
        );
    }

    pub fn push_clip(&mut self, bounds: Rect) {
        self.draw_list.push_clip(bounds);
    }

    pub fn pop_clip(&mut self) {
        self.draw_list.pop_clip();
    }

    pub fn push_translation(&mut self, offset: (f32, f32)) {
        self.draw_list.push_translation(offset);
    }

    pub fn pop_translation(&mut self) {
        self.draw_list.pop_translation();
    }

    pub fn request_next_frame(&mut self) {
        self.commands.request_next_frame = true;
    }
}

impl DrawSurface {
    pub fn new(scene: impl Fn(&mut DrawSurfaceContext) + 'static) -> Self {
        Self {
            id: WidgetId::default(),
            style: taffy::Style::default(),
            scene: Box::new(scene),
        }
    }

    pub fn fill(mut self) -> Self {
        self.style.size = taffy::prelude::Size {
            width: taffy::prelude::percent(1.0),
            height: taffy::prelude::percent(1.0),
        };
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.style.size.width = taffy::prelude::percent(1.0);
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style.size.height = taffy::prelude::percent(1.0);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.style.size.width = taffy::prelude::length(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.style.size.height = taffy::prelude::length(height);
        self
    }

    pub fn scene(&self, ctx: &mut DrawSurfaceContext) {
        (self.scene)(ctx);
    }
}

impl Widget for DrawSurface {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> taffy::Style {
        self.style.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let scale_factor = ctx.scale_factor;
        let elapsed_time = ctx.elapsed_time;
        let mut surface_ctx = DrawSurfaceContext {
            draw_list: ctx.draw_list,
            bounds,
            scale_factor,
            elapsed_time,
            commands: ctx.commands,
        };
        self.scene(&mut surface_ctx);
    }

    #[cfg(target_arch = "wasm32")]
    fn paint(&self, _ctx: &mut PaintContext) {}

    fn draw_surface(&self) -> Option<&crate::DrawSurface> {
        Some(self)
    }
}
