//! Text widget for displaying static text.

use std::borrow::Cow;

use crate::{
    current_theme, responsive_typography, AccessibilityInfo, AccessibilityRole, EventContext,
    PaintContext, Widget,
};
use bon::bon;
use sparsha_core::Color;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
use sparsha_text::{TextBreakMode, TextLayoutAlignment, TextLayoutOptions, TextStyle, TextWrap};
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

/// Overflow behavior for text painting when content exceeds the allocated bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextOverflow {
    #[default]
    Visible,
    Clip,
    Ellipsis,
}

/// A simple text display widget.
pub struct Text {
    id: WidgetId,
    content: String,
    color: Option<Color>,
    font_size: Option<f32>,
    line_height: Option<f32>,
    bold: bool,
    italic: bool,
    align: TextAlign,
    variant: TextVariant,
    fill_width: bool,
    wrap: TextWrap,
    break_mode: TextBreakMode,
    max_lines: Option<usize>,
    overflow: TextOverflow,
}

impl Text {
    fn with_content(content: String) -> Self {
        Self {
            id: WidgetId::default(),
            content,
            color: None,
            font_size: None,
            line_height: None,
            bold: false,
            italic: false,
            align: TextAlign::Left,
            variant: TextVariant::Body,
            fill_width: false,
            wrap: TextWrap::NoWrap,
            break_mode: TextBreakMode::Normal,
            max_lines: None,
            overflow: TextOverflow::Visible,
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

        if let Some(line_height) = self.line_height {
            style = style.with_line_height(line_height);
        }

        if self.bold {
            style = style.bold();
        }
        if self.italic {
            style = style.italic();
        }

        style
    }

    fn layout_alignment(&self) -> TextLayoutAlignment {
        match self.align {
            TextAlign::Left => TextLayoutAlignment::Start,
            TextAlign::Center => TextLayoutAlignment::Center,
            TextAlign::Right => TextLayoutAlignment::End,
        }
    }

    fn uses_block_layout(&self) -> bool {
        self.fill_width
            || self.wrap != TextWrap::NoWrap
            || self.max_lines.is_some()
            || matches!(self.overflow, TextOverflow::Ellipsis)
    }

    fn effective_max_lines(&self) -> Option<usize> {
        match self.overflow {
            TextOverflow::Ellipsis => Some(self.max_lines.unwrap_or(1).max(1)),
            _ => self.max_lines,
        }
    }

    fn clips_overflow(&self) -> bool {
        matches!(self.overflow, TextOverflow::Clip | TextOverflow::Ellipsis)
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
        line_height: Option<f32>,
        bold: Option<bool>,
        #[builder(default)] italic: bool,
        #[builder(default = TextAlign::Left)] align: TextAlign,
        #[builder(default)] fill_width: bool,
        #[builder(default = TextWrap::NoWrap)] wrap: TextWrap,
        #[builder(default = TextBreakMode::Normal)] break_mode: TextBreakMode,
        max_lines: Option<usize>,
        #[builder(default = TextOverflow::Visible)] overflow: TextOverflow,
    ) -> Self {
        let mut text = Self::with_content(content);
        text.variant = variant;
        text.color = color;
        text.font_size = font_size;
        text.line_height = line_height;
        text.bold = bold.unwrap_or(matches!(
            variant,
            TextVariant::Header | TextVariant::Subheader
        ));
        text.italic = italic;
        text.align = align;
        text.fill_width = fill_width;
        text.wrap = wrap;
        text.break_mode = break_mode;
        text.max_lines = max_lines;
        text.overflow = overflow;
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
                width: if self.fill_width
                    || self.wrap != TextWrap::NoWrap
                    || matches!(self.overflow, TextOverflow::Ellipsis)
                {
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

        if self.uses_block_layout() {
            let max_lines = self.effective_max_lines();
            let text: Cow<'_, str> = match self.overflow {
                TextOverflow::Ellipsis => {
                    let scaled_style = TextStyle {
                        font_size: style.font_size * ctx.scale_factor,
                        ..style.clone()
                    };
                    Cow::Owned(
                        ctx.text_system.ellipsize_with_options(
                            &self.content,
                            &scaled_style,
                            TextLayoutOptions::new()
                                .with_max_width(Some(bounds.width.max(0.0)))
                                .with_wrap(self.wrap)
                                .with_break_mode(self.break_mode)
                                .with_alignment(self.layout_alignment())
                                .with_max_lines(max_lines),
                        ),
                    )
                }
                _ => Cow::Borrowed(self.content.as_str()),
            };
            if self.clips_overflow() {
                ctx.push_clip(bounds);
            }
            ctx.draw_text_block(
                &text,
                &style,
                bounds,
                self.wrap,
                self.break_mode,
                self.layout_alignment(),
                max_lines,
            );
            if self.clips_overflow() {
                ctx.pop_clip();
            }
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

        if self.clips_overflow() {
            ctx.push_clip(bounds);
        }
        ctx.draw_text(&self.content, &style, x, y);
        if self.clips_overflow() {
            ctx.pop_clip();
        }
    }

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) {}

    fn focusable(&self) -> bool {
        false
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let style = self.text_style();
        let (w, h) = if self.uses_block_layout() {
            ctx.measure_text_layout(
                &self.content,
                &style,
                self.wrap,
                self.break_mode,
                self.layout_alignment(),
                self.effective_max_lines(),
            )
        } else {
            ctx.text.measure(&self.content, &style, None)
        };
        Some((w, h))
    }

    fn requires_post_layout_measurement(&self) -> bool {
        self.uses_block_layout()
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        (!self.content.is_empty())
            .then(|| AccessibilityInfo::new(AccessibilityRole::Label).label(self.content.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        set_current_theme, set_current_viewport, test_helpers::layout_bounds, PaintCommands,
        PaintContext, Theme, ViewportInfo,
    };
    use sparsha_input::FocusManager;
    use sparsha_layout::LayoutTree;
    use sparsha_render::{DrawCommand, DrawList};
    use sparsha_text::{TextBreakMode, TextSystem};

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
            .line_height(1.6)
            .color(Color::from_hex(0x22C55E))
            .build();
        let style = text.text_style();
        assert_eq!(style.font_size, 14.0);
        assert!((style.line_height - 1.6).abs() < 1e-5);
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
            .line_height(1.5)
            .bold(false)
            .italic(true)
            .align(TextAlign::Center)
            .fill_width(true)
            .wrap(TextWrap::Word)
            .break_mode(TextBreakMode::BreakWord)
            .max_lines(2)
            .overflow(TextOverflow::Clip)
            .build();

        assert_eq!(text.content, "Builder");
        assert_eq!(text.variant, TextVariant::Subheader);
        assert_eq!(text.color, Some(Color::from_hex(0x22C55E)));
        assert_eq!(text.font_size, Some(18.0));
        assert_eq!(text.line_height, Some(1.5));
        assert!(!text.bold);
        assert!(text.italic);
        assert_eq!(text.align, TextAlign::Center);
        assert!(text.fill_width);
        assert_eq!(text.wrap, TextWrap::Word);
        assert_eq!(text.break_mode, TextBreakMode::BreakWord);
        assert_eq!(text.max_lines, Some(2));
        assert_eq!(text.overflow, TextOverflow::Clip);
    }

    #[test]
    fn explicit_line_height_beats_default_text_style() {
        set_current_viewport(ViewportInfo::default());
        set_current_theme(Theme::default());

        let text = Text::builder()
            .content("Paragraph")
            .line_height(1.7)
            .build();

        assert!((text.text_style().line_height - 1.7).abs() < 1e-5);
    }

    #[test]
    fn fill_width_updates_layout_style() {
        let intrinsic = Text::builder().content("Intrinsic").build();
        let stretched = Text::builder().content("Block").fill_width(true).build();
        let wrapped = Text::builder()
            .content("Wrapped")
            .wrap(TextWrap::Word)
            .build();
        let ellipsized = Text::builder()
            .content("Ellipsized")
            .overflow(TextOverflow::Ellipsis)
            .build();

        assert_eq!(intrinsic.style().size.width, auto());
        assert_eq!(stretched.style().size.width, percent(1.0));
        assert_eq!(wrapped.style().size.width, percent(1.0));
        assert_eq!(ellipsized.style().size.width, percent(1.0));
    }

    #[test]
    fn block_text_clip_overflow_emits_clip_commands() {
        let mut draw_list = DrawList::new();
        let layout_tree = LayoutTree::new();
        let focus = FocusManager::new();
        let mut text_system = TextSystem::new_headless();
        let mut commands = PaintCommands::default();
        let text = Text::builder()
            .content("This block should clip to its bounds.")
            .fill_width(true)
            .wrap(TextWrap::Word)
            .overflow(TextOverflow::Clip)
            .build();

        let mut ctx = PaintContext {
            draw_list: &mut draw_list,
            layout: layout_bounds(0.0, 0.0, 120.0, 40.0),
            layout_tree: &layout_tree,
            focus: &focus,
            widget_id: text.id(),
            scale_factor: 1.0,
            text_system: &mut text_system,
            elapsed_time: 0.0,
            commands: &mut commands,
        };
        text.paint(&mut ctx);

        assert!(matches!(
            draw_list.commands().first(),
            Some(DrawCommand::PushClip { .. })
        ));
        assert!(matches!(
            draw_list.commands().last(),
            Some(DrawCommand::PopClip)
        ));
    }

    #[test]
    fn block_text_ellipsis_emits_truncated_text_run() {
        let mut draw_list = DrawList::new();
        let layout_tree = LayoutTree::new();
        let focus = FocusManager::new();
        let mut text_system = TextSystem::new_headless();
        let mut commands = PaintCommands::default();
        let text = Text::builder()
            .content("This block should ellipsize instead of painting its full label.")
            .overflow(TextOverflow::Ellipsis)
            .align(TextAlign::Center)
            .build();

        let mut ctx = PaintContext {
            draw_list: &mut draw_list,
            layout: layout_bounds(0.0, 0.0, 140.0, 36.0),
            layout_tree: &layout_tree,
            focus: &focus,
            widget_id: text.id(),
            scale_factor: 1.0,
            text_system: &mut text_system,
            elapsed_time: 0.0,
            commands: &mut commands,
        };
        text.paint(&mut ctx);

        let run = draw_list
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::TextRun { run } => Some(run),
                _ => None,
            })
            .expect("expected text run");

        assert!(run.text.ends_with('\u{2026}'));
        assert_eq!(run.max_width, Some(140.0));
        assert_eq!(run.max_lines, Some(1));
        assert_eq!(run.alignment, TextLayoutAlignment::Center);
    }
}
