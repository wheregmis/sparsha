//! Text widget for displaying static text.

use crate::{
    current_theme, responsive_typography, AccessibilityInfo, AccessibilityRole, EventContext,
    PaintContext, Widget,
};
use bon::bon;
use sparsha_core::Color;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
use sparsha_text::TextStyle;
use taffy::prelude::*;

/// Text alignment options.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// Responsive typography variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextVariant {
    #[default]
    Body,
    Header,
    Subheader,
    Caption,
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
    variant: TextVariant,
    fill_width: bool,
}

impl Text {
    fn with_content(content: String) -> Self {
        Self {
            id: WidgetId::default(),
            content,
            color: None,
            font_size: None,
            bold: false,
            italic: false,
            align: TextAlign::Left,
            variant: TextVariant::Body,
            fill_width: false,
        }
    }

    fn text_style(&self) -> TextStyle {
        let theme = current_theme();
        let typography = responsive_typography(&theme);
        let resolved_size = self.font_size.unwrap_or(match self.variant {
            TextVariant::Body => typography.body_size,
            TextVariant::Header => typography.title_size,
            TextVariant::Subheader => typography.subheader_size,
            TextVariant::Caption => typography.small_size,
        });
        let resolved_color = self.color.unwrap_or(match self.variant {
            TextVariant::Caption => theme.colors.text_muted,
            _ => theme.colors.text_primary,
        });
        let mut style = TextStyle::default()
            .with_family(theme.typography.font_family.clone())
            .with_size(resolved_size)
            .with_color(resolved_color);

        if self.bold {
            style = style.bold();
        }
        if self.italic {
            style = style.italic();
        }

        style
    }
}

#[bon]
impl Text {
    #[builder(
        start_fn(name = builder, vis = "pub"),
        finish_fn(name = build, vis = "pub"),
        builder_type(name = TextBuilder, vis = "pub"),
        state_mod(vis = "pub")
    )]
    fn builder_init(
        #[builder(into)] content: String,
        #[builder(default = TextVariant::Body)] variant: TextVariant,
        color: Option<Color>,
        font_size: Option<f32>,
        bold: Option<bool>,
        #[builder(default)] italic: bool,
        #[builder(default = TextAlign::Left)] align: TextAlign,
        #[builder(default)] fill_width: bool,
    ) -> Self {
        let mut text = Self::with_content(content);
        text.variant = variant;
        text.color = color;
        text.font_size = font_size;
        text.bold = bold.unwrap_or(matches!(
            variant,
            TextVariant::Header | TextVariant::Subheader
        ));
        text.italic = italic;
        text.align = align;
        text.fill_width = fill_width;
        text
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
        Style {
            size: Size {
                width: if self.fill_width {
                    percent(1.0)
                } else {
                    auto()
                },
                height: auto(),
            },
            ..Style::default()
        }
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
    use crate::{set_current_theme, set_current_viewport, Theme, ViewportInfo};

    #[test]
    fn text_defaults_follow_current_theme() {
        let mut theme = Theme::default();
        theme.typography.body_size = 19.0;
        theme.colors.text_primary = Color::from_hex(0x334155);
        set_current_viewport(ViewportInfo::default());
        set_current_theme(theme.clone());

        let text = Text::builder().content("Theme text").build();
        let style = text.text_style();
        assert_eq!(style.font_size, 19.0);
        assert_eq!(style.color, theme.colors.text_primary);
    }

    #[test]
    fn explicit_overrides_beat_theme_defaults() {
        let mut theme = Theme::default();
        theme.typography.body_size = 21.0;
        set_current_viewport(ViewportInfo::default());
        set_current_theme(theme);

        let text = Text::builder()
            .content("Override")
            .font_size(14.0)
            .color(Color::from_hex(0x22C55E))
            .build();
        let style = text.text_style();
        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.color, Color::from_hex(0x22C55E));
    }

    #[test]
    fn header_and_caption_follow_responsive_typography() {
        let mut theme = Theme::default();
        theme.typography.title_size = 24.0;
        theme.typography.small_size = 12.0;
        set_current_theme(theme.clone());
        set_current_viewport(ViewportInfo::new(390.0, 844.0));

        let header = Text::builder()
            .content("Header")
            .variant(TextVariant::Header)
            .build();
        let caption = Text::builder()
            .content("Caption")
            .variant(TextVariant::Caption)
            .build();
        assert!(header.text_style().font_size < 24.0);
        assert!(caption.text_style().font_size <= 12.0);
        assert_eq!(caption.text_style().color, theme.colors.text_muted);
        assert!(header.bold);
    }

    #[test]
    fn builder_sets_explicit_configuration() {
        let text = Text::builder()
            .content("Builder")
            .variant(TextVariant::Subheader)
            .color(Color::from_hex(0x22C55E))
            .font_size(18.0)
            .bold(false)
            .italic(true)
            .align(TextAlign::Center)
            .fill_width(true)
            .build();

        assert_eq!(text.content, "Builder");
        assert_eq!(text.variant, TextVariant::Subheader);
        assert_eq!(text.color, Some(Color::from_hex(0x22C55E)));
        assert_eq!(text.font_size, Some(18.0));
        assert!(!text.bold);
        assert!(text.italic);
        assert_eq!(text.align, TextAlign::Center);
        assert!(text.fill_width);
    }

    #[test]
    fn fill_width_updates_layout_style() {
        let intrinsic = Text::builder().content("Intrinsic").build();
        let stretched = Text::builder().content("Block").fill_width(true).build();

        assert_eq!(intrinsic.style().size.width, auto());
        assert_eq!(stretched.style().size.width, percent(1.0));
    }
}
