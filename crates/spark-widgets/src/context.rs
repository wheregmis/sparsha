//! Context types passed to widgets during layout, paint, and events.

use spark_core::{Color, GlyphInstance, Rect};
use spark_input::FocusManager;
use spark_layout::{ComputedLayout, LayoutTree, WidgetId};
use spark_render::DrawList;
use spark_text::{TextStyle, TextSystem};
use wgpu::{Device, Queue};

/// Context for layout measurement.
pub struct LayoutContext<'a> {
    /// The text system for measuring text.
    pub text: &'a mut TextSystem,
    /// Available width constraint.
    pub max_width: Option<f32>,
    /// Available height constraint.
    pub max_height: Option<f32>,
}

impl<'a> LayoutContext<'a> {
    /// Measure text with the current constraints.
    pub fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        self.text.measure(text, style, self.max_width)
    }
}

/// Context for painting widgets.
pub struct PaintContext<'a> {
    /// The draw list to paint to.
    pub draw_list: &'a mut DrawList,
    /// The computed layout for this widget.
    pub layout: ComputedLayout,
    /// The layout tree for querying child layouts.
    pub layout_tree: &'a LayoutTree,
    /// The focus manager (for focus state).
    pub focus: &'a FocusManager,
    /// Current widget ID.
    pub widget_id: WidgetId,
    /// Scale factor for HiDPI.
    pub scale_factor: f32,
    /// The text system for shaping text.
    pub text_system: &'a mut TextSystem,
    /// The GPU device.
    pub device: &'a Device,
    /// The GPU queue.
    pub queue: &'a Queue,
    /// Elapsed time in seconds (for animations like cursor blinking).
    pub elapsed_time: f32,
}

impl<'a> PaintContext<'a> {
    /// Get the widget's bounds.
    pub fn bounds(&self) -> Rect {
        self.layout.bounds
    }

    /// Check if this widget has keyboard focus.
    pub fn has_focus(&self) -> bool {
        self.focus.has_focus(self.widget_id)
    }

    /// Draw a filled rectangle.
    /// Bounds are in physical pixels.
    pub fn fill_rect(&mut self, bounds: Rect, color: Color) {
        self.draw_list.rect(bounds, color);
    }

    /// Draw a rounded rectangle.
    /// Bounds and radius are in physical pixels.
    pub fn fill_rounded_rect(&mut self, bounds: Rect, color: Color, radius: f32) {
        // Scale radius for HiDPI
        let scaled_radius = radius * self.scale_factor;
        self.draw_list.rounded_rect(bounds, color, scaled_radius);
    }

    /// Draw a rectangle with a border.
    /// Bounds, radius, and border_width are in physical pixels.
    pub fn fill_bordered_rect(
        &mut self,
        bounds: Rect,
        color: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        // Scale radius and border for HiDPI
        let scaled_radius = radius * self.scale_factor;
        let scaled_border = border_width * self.scale_factor;
        self.draw_list
            .bordered_rect(bounds, color, scaled_radius, scaled_border, border_color);
    }

    /// Push a clip rectangle.
    pub fn push_clip(&mut self, bounds: Rect) {
        self.draw_list.push_clip(bounds);
    }

    /// Pop the clip rectangle.
    pub fn pop_clip(&mut self) {
        self.draw_list.pop_clip();
    }

    /// Push a translation offset for subsequent draw commands.
    /// The offset is in physical pixels.
    pub fn push_translation(&mut self, offset: (f32, f32)) {
        self.draw_list.push_translation(offset);
    }

    /// Pop the current translation offset.
    pub fn pop_translation(&mut self) {
        self.draw_list.pop_translation();
    }

    /// Draw text at the specified position.
    ///
    /// The text is shaped using the provided style and drawn with its
    /// top-left corner at (x, y). Coordinates are in physical pixels.
    pub fn draw_text(&mut self, text: &str, style: &TextStyle, x: f32, y: f32) {
        if text.is_empty() {
            return;
        }

        // Scale font size for HiDPI rendering
        let scaled_style = TextStyle {
            font_size: style.font_size * self.scale_factor,
            ..style.clone()
        };

        let shaped = self
            .text_system
            .shape(self.device, self.queue, text, &scaled_style, None);

        // Offset all glyphs by the given position
        let glyphs: Vec<GlyphInstance> = shaped
            .glyphs
            .iter()
            .map(|g| GlyphInstance {
                pos: [g.pos[0] + x, g.pos[1] + y],
                ..*g
            })
            .collect();

        self.draw_list.text(glyphs);
    }

    /// Draw text centered within the given bounds.
    ///
    /// The text is horizontally and vertically centered within the bounds.
    /// Bounds are in physical pixels.
    pub fn draw_text_centered(&mut self, text: &str, style: &TextStyle, bounds: Rect) {
        if text.is_empty() {
            return;
        }

        // Measure text at scaled size to get dimensions
        let (text_width, text_height) = self.measure_text(text, style);

        // Calculate centered position
        let x = bounds.x + (bounds.width - text_width) / 2.0;
        let y = bounds.y + (bounds.height - text_height) / 2.0;

        self.draw_text(text, style, x, y);
    }

    /// Draw text left-aligned within the given bounds, vertically centered.
    ///
    /// Useful for text inputs and labels. Bounds are in physical pixels.
    pub fn draw_text_aligned(
        &mut self,
        text: &str,
        style: &TextStyle,
        bounds: Rect,
        padding_left: f32,
    ) {
        if text.is_empty() {
            return;
        }

        // Measure text at scaled size to get dimensions
        let (_text_width, text_height) = self.measure_text(text, style);

        // Calculate position: left-aligned with padding, vertically centered
        // Padding is also in physical pixels since bounds are
        let x = bounds.x + padding_left;
        let y = bounds.y + (bounds.height - text_height) / 2.0;

        self.draw_text(text, style, x, y);
    }

    /// Measure text dimensions without drawing.
    /// Returns dimensions in physical pixels (scaled by scale_factor).
    pub fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        // Scale font size for HiDPI measurement
        let scaled_style = TextStyle {
            font_size: style.font_size * self.scale_factor,
            ..style.clone()
        };
        self.text_system.measure(text, &scaled_style, None)
    }
}

/// Context for handling events.
pub struct EventContext<'a> {
    /// The computed layout for this widget.
    pub layout: ComputedLayout,
    /// The layout tree for hit testing children.
    pub layout_tree: &'a LayoutTree,
    /// Focus manager.
    pub focus: &'a mut FocusManager,
    /// Current widget ID.
    pub widget_id: WidgetId,
    /// Whether this widget has pointer capture.
    pub has_capture: bool,
}

impl<'a> EventContext<'a> {
    /// Get the widget's bounds.
    pub fn bounds(&self) -> Rect {
        self.layout.bounds
    }

    /// Check if this widget has keyboard focus.
    pub fn has_focus(&self) -> bool {
        self.focus.has_focus(self.widget_id)
    }

    /// Request keyboard focus for this widget.
    pub fn request_focus(&mut self) {
        self.focus.set_focus(self.widget_id);
    }

    /// Release keyboard focus.
    pub fn release_focus(&mut self) {
        if self.has_focus() {
            self.focus.clear_focus();
        }
    }

    /// Check if a point is inside this widget's bounds.
    pub fn contains(&self, pos: glam::Vec2) -> bool {
        self.layout.bounds.contains(pos)
    }

    /// Convert a point to local coordinates.
    pub fn to_local(&self, pos: glam::Vec2) -> glam::Vec2 {
        glam::Vec2::new(pos.x - self.layout.bounds.x, pos.y - self.layout.bounds.y)
    }
}
