//! Text widget for displaying static text.

use crate::{EventContext, PaintContext, Widget};
use spark_core::Color;
use spark_input::InputEvent;
use spark_layout::WidgetId;
use spark_text::TextStyle;
use taffy::prelude::*;

/// Text alignment options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// A simple text display widget.
pub struct Text {
    id: WidgetId,
    content: String,
    color: Color,
    font_size: f32,
    bold: bool,
    italic: bool,
    align: TextAlign,
}

impl Text {
    /// Create a new text widget with the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: WidgetId::default(),
            content: content.into(),
            color: Color::from_hex(0x1F2937), // Default dark gray text
            font_size: 16.0,
            bold: false,
            italic: false,
            align: TextAlign::Left,
        }
    }

    /// Set the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the font size.
    pub fn size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Make the text bold.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Make the text italic.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Set text alignment.
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Center-align the text.
    pub fn center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }

    /// Right-align the text.
    pub fn right(mut self) -> Self {
        self.align = TextAlign::Right;
        self
    }

    /// Create a header-style text (larger, bold).
    pub fn header(content: impl Into<String>) -> Self {
        Self::new(content).size(24.0).bold()
    }

    /// Create a subheader-style text.
    pub fn subheader(content: impl Into<String>) -> Self {
        Self::new(content).size(18.0).bold()
    }

    /// Create a small/caption-style text.
    pub fn caption(content: impl Into<String>) -> Self {
        Self::new(content)
            .size(12.0)
            .color(Color::from_hex(0x6B7280))
    }

    fn text_style(&self) -> TextStyle {
        let mut style = TextStyle::default()
            .with_size(self.font_size)
            .with_color(self.color);

        if self.bold {
            style = style.bold();
        }
        if self.italic {
            style = style.italic();
        }

        style
    }
}

impl Widget for Text {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style::default()
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let style = self.text_style();

        if self.content.is_empty() {
            return;
        }

        // Measure text for alignment
        let (text_width, text_height) = ctx.measure_text(&self.content, &style);

        // Calculate x position based on alignment
        let x = match self.align {
            TextAlign::Left => bounds.x,
            TextAlign::Center => bounds.x + (bounds.width - text_width) / 2.0,
            TextAlign::Right => bounds.x + bounds.width - text_width,
        };

        // Vertically center text within bounds
        let y = bounds.y + (bounds.height - text_height) / 2.0;

        ctx.draw_text(&self.content, &style, x, y);
    }

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) {}

    fn focusable(&self) -> bool {
        false
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let style = self.text_style();
        let (w, h) = ctx.text.measure(&self.content, &style, None);
        Some((w, h))
    }
}
