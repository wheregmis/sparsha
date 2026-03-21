//! Text widget for displaying static text.

use crate::{
    current_theme, AccessibilityInfo, AccessibilityRole, EventContext, PaintContext, Widget,
};
use sparsh_core::Color;
use sparsh_input::InputEvent;
use sparsh_layout::WidgetId;
use sparsh_text::TextStyle;
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
    color: Option<Color>,
    font_size: Option<f32>,
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
            color: None,
            font_size: None,
            bold: false,
            italic: false,
            align: TextAlign::Left,
        }
    }

    /// Set the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the font size.
    pub fn size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
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
        let theme = current_theme();
        Self::new(content)
            .size(theme.typography.small_size)
            .color(theme.colors.text_muted)
    }

    fn text_style(&self) -> TextStyle {
        let theme = current_theme();
        let mut style = TextStyle::default()
            .with_family(theme.typography.font_family.clone())
            .with_size(self.font_size.unwrap_or(theme.typography.body_size))
            .with_color(self.color.unwrap_or(theme.colors.text_primary));

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

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        (!self.content.is_empty())
            .then(|| AccessibilityInfo::new(AccessibilityRole::Label).label(self.content.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{set_current_theme, Theme};

    #[test]
    fn text_defaults_follow_current_theme() {
        let mut theme = Theme::default();
        theme.typography.body_size = 19.0;
        theme.colors.text_primary = Color::from_hex(0x334155);
        set_current_theme(theme.clone());

        let text = Text::new("Theme text");
        let style = text.text_style();
        assert_eq!(style.font_size, 19.0);
        assert_eq!(style.color, theme.colors.text_primary);
    }

    #[test]
    fn explicit_overrides_beat_theme_defaults() {
        let mut theme = Theme::default();
        theme.typography.body_size = 21.0;
        set_current_theme(theme);

        let text = Text::new("Override")
            .size(14.0)
            .color(Color::from_hex(0x22C55E));
        let style = text.text_style();
        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.color, Color::from_hex(0x22C55E));
    }
}
