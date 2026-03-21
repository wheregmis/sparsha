//! Draw commands that represent what to render.

use sparsh_core::{Color, GlyphInstance, Rect};
use sparsh_text::TextStyle;

/// A logical text run that can be consumed by different render backends.
#[derive(Clone, Debug)]
pub struct TextRun {
    /// UTF-8 text content.
    pub text: String,
    /// Text styling attributes.
    pub style: TextStyle,
    /// Top-left origin in render-space pixels.
    pub position: (f32, f32),
}

/// A single draw command representing a primitive to render.
#[derive(Clone, Debug)]
pub enum DrawCommand {
    /// Draw a filled rectangle with optional rounded corners.
    Rect {
        bounds: Rect,
        color: Color,
        corner_radius: f32,
        border_width: f32,
        border_color: Color,
    },
    /// Draw a line segment with thickness.
    Line {
        start: (f32, f32),
        end: (f32, f32),
        thickness: f32,
        color: Color,
    },
    /// Draw text glyphs.
    Text { glyphs: Vec<GlyphInstance> },
    /// Draw a text run (backend-neutral text command).
    TextRun { run: TextRun },
    /// Push a clip rectangle (future draw commands will be clipped).
    PushClip { bounds: Rect },
    /// Pop the current clip rectangle.
    PopClip,
    /// Push a translation offset (affects all subsequent draw commands).
    PushTranslation { offset: (f32, f32) },
    /// Pop the current translation offset.
    PopTranslation,
}

impl DrawCommand {
    /// Create a simple filled rectangle.
    pub fn rect(bounds: Rect, color: Color) -> Self {
        Self::Rect {
            bounds,
            color,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    /// Create a rounded rectangle.
    pub fn rounded_rect(bounds: Rect, color: Color, radius: f32) -> Self {
        Self::Rect {
            bounds,
            color,
            corner_radius: radius,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    /// Create a rectangle with a border.
    pub fn bordered_rect(
        bounds: Rect,
        color: Color,
        corner_radius: f32,
        border_width: f32,
        border_color: Color,
    ) -> Self {
        Self::Rect {
            bounds,
            color,
            corner_radius,
            border_width,
            border_color,
        }
    }

    /// Create a line segment.
    pub fn line(start: (f32, f32), end: (f32, f32), thickness: f32, color: Color) -> Self {
        Self::Line {
            start,
            end,
            thickness,
            color,
        }
    }
}

/// A list of draw commands to be rendered in order.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    commands: Vec<DrawCommand>,
}

impl DrawList {
    /// Create an empty draw list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a draw command to the list.
    pub fn push(&mut self, command: DrawCommand) {
        self.commands.push(command);
    }

    /// Draw a filled rectangle.
    pub fn rect(&mut self, bounds: Rect, color: Color) {
        self.push(DrawCommand::rect(bounds, color));
    }

    /// Draw a rounded rectangle.
    pub fn rounded_rect(&mut self, bounds: Rect, color: Color, radius: f32) {
        self.push(DrawCommand::rounded_rect(bounds, color, radius));
    }

    /// Draw a rectangle with a border.
    pub fn bordered_rect(
        &mut self,
        bounds: Rect,
        color: Color,
        corner_radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        self.push(DrawCommand::bordered_rect(
            bounds,
            color,
            corner_radius,
            border_width,
            border_color,
        ));
    }

    /// Draw a line segment.
    pub fn line(&mut self, start: (f32, f32), end: (f32, f32), thickness: f32, color: Color) {
        if thickness > 0.0 {
            self.push(DrawCommand::line(start, end, thickness, color));
        }
    }

    /// Draw text glyphs.
    pub fn text(&mut self, glyphs: Vec<GlyphInstance>) {
        if !glyphs.is_empty() {
            self.push(DrawCommand::Text { glyphs });
        }
    }

    /// Draw a logical text run.
    pub fn text_run(&mut self, text: impl Into<String>, style: TextStyle, x: f32, y: f32) {
        let text = text.into();
        if !text.is_empty() {
            self.push(DrawCommand::TextRun {
                run: TextRun {
                    text,
                    style,
                    position: (x, y),
                },
            });
        }
    }

    /// Push a clip rectangle.
    pub fn push_clip(&mut self, bounds: Rect) {
        self.push(DrawCommand::PushClip { bounds });
    }

    /// Pop the current clip rectangle.
    pub fn pop_clip(&mut self) {
        self.push(DrawCommand::PopClip);
    }

    /// Push a translation offset for subsequent draw commands.
    pub fn push_translation(&mut self, offset: (f32, f32)) {
        self.push(DrawCommand::PushTranslation { offset });
    }

    /// Pop the current translation offset.
    pub fn pop_translation(&mut self) {
        self.push(DrawCommand::PopTranslation);
    }

    /// Get all commands.
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Clear all commands.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the number of commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore = "perf smoke"]
    fn perf_smoke_draw_list_encoding() {
        let start = Instant::now();
        let mut draw_list = DrawList::new();
        let text_style = TextStyle::default().with_family("Inter").with_size(14.0);

        for row in 0..400 {
            let y = row as f32 * 18.0;
            draw_list.push_clip(Rect::new(0.0, y, 960.0, 18.0));
            draw_list.push_translation((0.0, y));
            draw_list.bordered_rect(
                Rect::new(0.0, 0.0, 960.0, 16.0),
                if row % 2 == 0 {
                    Color::from_hex(0xE2E8F0)
                } else {
                    Color::from_hex(0xCBD5E1)
                },
                4.0,
                1.0,
                Color::from_hex(0x94A3B8),
            );
            draw_list.line((0.0, 16.0), (960.0, 16.0), 1.0, Color::from_hex(0x475569));
            draw_list.text_run(format!("Row {}", row + 1), text_style.clone(), 12.0, 2.0);
            draw_list.pop_translation();
            draw_list.pop_clip();
        }

        let rect_count = draw_list
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::Rect { .. }))
            .count();
        let text_count = draw_list
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::TextRun { .. }))
            .count();
        let elapsed = start.elapsed();

        println!(
            "render perf smoke: encoded {} commands ({} rects, {} text runs) in {:?}",
            draw_list.len(),
            rect_count,
            text_count,
            elapsed
        );
        assert_eq!(rect_count, 400);
        assert_eq!(text_count, 400);
    }
}
