//! Text input widget.

use crate::{EventContext, EventResponse, PaintContext, Widget};
use spark_core::Color;
use spark_input::{shortcuts, InputEvent, Key};
use spark_layout::WidgetId;
use spark_text::TextStyle;
use std::cell::RefCell;
use taffy::prelude::*;

/// Callback type for text change and submit handlers.
type TextInputCallback = Box<dyn FnMut(&str) + Send + Sync>;

/// Style configuration for text input.
#[derive(Clone, Debug)]
pub struct TextInputStyle {
    pub background: Color,
    pub background_focused: Color,
    pub text_color: Color,
    pub placeholder_color: Color,
    pub border_color: Color,
    pub border_color_focused: Color,
    pub border_width: f32,
    pub corner_radius: f32,
    pub padding_h: f32,
    pub padding_v: f32,
    pub font_size: f32,
    pub min_width: f32,
    pub min_height: f32,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            background: Color::WHITE,
            background_focused: Color::WHITE,
            text_color: Color::from_hex(0x1F2937),
            placeholder_color: Color::from_hex(0x9CA3AF),
            border_color: Color::from_hex(0xD1D5DB),
            border_color_focused: Color::from_hex(0x3B82F6),
            border_width: 1.0,
            corner_radius: 6.0,
            padding_h: 12.0,
            padding_v: 8.0,
            font_size: 14.0,
            min_width: 180.0,
            min_height: 36.0,
        }
    }
}

/// A single-line text input widget.
pub struct TextInput {
    id: WidgetId,
    value: String,
    placeholder: String,
    style: TextInputStyle,
    cursor_pos: usize,
    selection_start: Option<usize>,
    on_change: Option<TextInputCallback>,
    on_submit: Option<TextInputCallback>,
    fill_width: bool,
    prefix_widths: RefCell<Vec<(usize, f32)>>,
}

impl TextInput {
    /// Create a new text input.
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            value: String::new(),
            placeholder: String::new(),
            style: TextInputStyle::default(),
            cursor_pos: 0,
            selection_start: None,
            on_change: None,
            on_submit: None,
            fill_width: false,
            prefix_widths: RefCell::new(vec![(0, 0.0)]),
        }
    }

    /// Set the initial value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self.cursor_pos = self.value.len();
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the change handler.
    pub fn on_change(mut self, handler: impl FnMut(&str) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set the submit handler (called on Enter).
    pub fn on_submit(mut self, handler: impl FnMut(&str) + Send + Sync + 'static) -> Self {
        self.on_submit = Some(Box::new(handler));
        self
    }

    /// Set the style.
    pub fn with_style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self
    }

    /// Stretch to fill the parent's available width.
    pub fn fill_width(mut self) -> Self {
        self.fill_width = true;
        self
    }

    /// Get the current value.
    pub fn get_value(&self) -> &str {
        &self.value
    }

    fn insert_char(&mut self, c: char) {
        self.delete_selection();
        self.value.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.fire_change();
    }

    #[allow(dead_code)]
    fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.value.insert_str(self.cursor_pos, s);
        self.cursor_pos += s.len();
        self.fire_change();
    }

    fn delete_selection(&mut self) {
        if let Some(start) = self.selection_start.take() {
            let (from, to) = if start < self.cursor_pos {
                (start, self.cursor_pos)
            } else {
                (self.cursor_pos, start)
            };
            self.value.drain(from..to);
            self.cursor_pos = from;
        }
    }

    fn backspace(&mut self) {
        if self.selection_start.is_some() {
            self.delete_selection();
            self.fire_change();
        } else if self.cursor_pos > 0 {
            // Find the previous character boundary
            let prev = self.value[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.drain(prev..self.cursor_pos);
            self.cursor_pos = prev;
            self.fire_change();
        }
    }

    fn delete(&mut self) {
        if self.selection_start.is_some() {
            self.delete_selection();
            self.fire_change();
        } else if self.cursor_pos < self.value.len() {
            // Find the next character boundary
            let next = self.value[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.value.len());
            self.value.drain(self.cursor_pos..next);
            self.fire_change();
        }
    }

    fn move_cursor_left(&mut self, shift: bool) {
        if !shift {
            self.selection_start = None;
        } else if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_pos);
        }

        if self.cursor_pos > 0 {
            self.cursor_pos = self.value[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    fn move_cursor_right(&mut self, shift: bool) {
        if !shift {
            self.selection_start = None;
        } else if self.selection_start.is_none() {
            self.selection_start = Some(self.cursor_pos);
        }

        if self.cursor_pos < self.value.len() {
            self.cursor_pos = self.value[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.value.len());
        }
    }

    fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor_pos = self.value.len();
    }

    fn fire_change(&mut self) {
        if let Some(handler) = &mut self.on_change {
            handler(&self.value);
        }
    }

    fn compute_prefix_widths(
        text: &str,
        mut measure_width: impl FnMut(&str) -> f32,
    ) -> Vec<(usize, f32)> {
        if text.is_empty() {
            return vec![(0, 0.0)];
        }

        let mut boundaries = Vec::with_capacity(text.chars().count() + 1);
        boundaries.push(0);
        for (idx, _) in text.char_indices().skip(1) {
            boundaries.push(idx);
        }
        if boundaries.last().copied() != Some(text.len()) {
            boundaries.push(text.len());
        }

        boundaries
            .into_iter()
            .map(|idx| (idx, measure_width(&text[..idx])))
            .collect()
    }

    fn update_prefix_width_cache_with_paint_ctx(&self, ctx: &mut PaintContext, style: &TextStyle) {
        let cache =
            Self::compute_prefix_widths(&self.value, |prefix| ctx.measure_text(prefix, style).0);
        *self.prefix_widths.borrow_mut() = cache;
    }

    fn update_prefix_width_cache_with_layout_ctx(
        &self,
        ctx: &mut crate::LayoutContext,
        style: &TextStyle,
    ) {
        let cache =
            Self::compute_prefix_widths(&self.value, |prefix| ctx.measure_text(prefix, style).0);
        *self.prefix_widths.borrow_mut() = cache;
    }

    fn cursor_index_for_x(&self, x: f32) -> usize {
        if self.value.is_empty() {
            return 0;
        }
        let prefix = self.prefix_widths.borrow();
        if prefix.is_empty() {
            return self.value.len();
        }

        if x <= 0.0 {
            return 0;
        }
        if let Some((last_idx, last_x)) = prefix.last() {
            if x >= *last_x {
                return *last_idx;
            }
        }

        let mut best = (self.value.len(), f32::MAX);
        for (idx, width) in prefix.iter() {
            let dist = (*width - x).abs();
            if dist < best.1 {
                best = (*idx, dist);
            }
        }
        best.0
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TextInput {
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
            padding: Rect {
                left: length(self.style.padding_h),
                right: length(self.style.padding_h),
                top: length(self.style.padding_v),
                bottom: length(self.style.padding_v),
            },
            min_size: Size {
                width: length(self.style.min_width),
                height: length(self.style.min_height),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let focused = ctx.has_focus();
        let scale = ctx.scale_factor;

        let bg = if focused {
            self.style.background_focused
        } else {
            self.style.background
        };

        let border = if focused {
            self.style.border_color_focused
        } else {
            self.style.border_color
        };

        // Draw background
        ctx.fill_bordered_rect(
            bounds,
            bg,
            self.style.corner_radius,
            self.style.border_width,
            border,
        );

        // Focus ring (scale the offset values)
        if focused {
            let offset = 2.0 * scale;
            let focus_bounds = spark_core::Rect::new(
                bounds.x - offset,
                bounds.y - offset,
                bounds.width + offset * 2.0,
                bounds.height + offset * 2.0,
            );
            ctx.fill_bordered_rect(
                focus_bounds,
                Color::TRANSPARENT,
                self.style.corner_radius + 2.0,
                2.0,
                Color::from_hex(0x60A5FA).with_alpha(0.5),
            );
        }

        // Calculate text area (inside padding) - scale padding for physical pixels
        let padding_h = self.style.padding_h * scale;
        let text_x = bounds.x + padding_h;
        let text_width = bounds.width - padding_h * 2.0;

        // Create text style (font size is in logical pixels, will be scaled by draw_text)
        let text_style = TextStyle::default()
            .with_size(self.style.font_size)
            .with_color(self.style.text_color);

        let placeholder_style = TextStyle::default()
            .with_size(self.style.font_size)
            .with_color(self.style.placeholder_color);

        self.update_prefix_width_cache_with_paint_ctx(ctx, &text_style);

        // Measure text height for vertical centering
        let (_, text_height) = ctx.measure_text("Ay", &text_style);
        let text_y = bounds.y + (bounds.height - text_height) / 2.0;

        // Draw placeholder or value
        if self.value.is_empty() {
            // Draw placeholder text
            if !self.placeholder.is_empty() {
                ctx.draw_text(&self.placeholder, &placeholder_style, text_x, text_y);
            }
        } else {
            // Draw selection highlight if any
            if let Some(sel_start) = self.selection_start {
                let (start, end) = if sel_start < self.cursor_pos {
                    (sel_start, self.cursor_pos)
                } else {
                    (self.cursor_pos, sel_start)
                };

                // Measure text before selection start
                let text_before_sel = &self.value[..start];
                let (sel_x_start, _) = ctx.measure_text(text_before_sel, &text_style);

                // Measure selected text
                let selected_text = &self.value[start..end];
                let (sel_width, _) = ctx.measure_text(selected_text, &text_style);

                // Draw selection rectangle
                if sel_width > 0.0 {
                    let sel_rect = spark_core::Rect::new(
                        text_x + sel_x_start,
                        text_y,
                        sel_width.min(text_width - sel_x_start),
                        text_height,
                    );
                    ctx.fill_rect(sel_rect, Color::from_hex(0x3B82F6).with_alpha(0.3));
                }
            }

            // Draw the text value
            ctx.draw_text(&self.value, &text_style, text_x, text_y);
        }

        // Draw cursor when focused
        if focused {
            // Blink cursor at ~2Hz
            let cursor_visible = (ctx.elapsed_time * 2.0).fract() < 0.5;

            if cursor_visible {
                // Measure text up to cursor position
                let text_before_cursor = &self.value[..self.cursor_pos];
                let (cursor_x_offset, _) = ctx.measure_text(text_before_cursor, &text_style);

                let cursor_x = text_x + cursor_x_offset;
                let cursor_height = text_height;

                // Draw cursor line (scale cursor width)
                let cursor_width = 2.0 * scale;
                let cursor_rect =
                    spark_core::Rect::new(cursor_x, text_y, cursor_width, cursor_height);
                ctx.fill_rect(cursor_rect, self.style.text_color);
            }
        }
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) -> EventResponse {
        match event {
            InputEvent::PointerDown { pos, .. } => {
                if ctx.contains(*pos) {
                    ctx.request_focus();
                    let local = ctx.to_local(*pos);
                    let click_x = (local.x - self.style.padding_h).max(0.0);
                    self.cursor_pos = self.cursor_index_for_x(click_x);
                    self.selection_start = None;
                    return EventResponse::focus();
                }
                EventResponse::default()
            }
            InputEvent::KeyDown { event } => {
                if !ctx.has_focus() {
                    return EventResponse::default();
                }

                use spark_input::NamedKey;

                // Handle shortcuts
                if shortcuts::is_select_all(event) {
                    self.select_all();
                    return EventResponse::handled();
                }

                if shortcuts::is_backspace(event) {
                    self.backspace();
                    return EventResponse::handled();
                }

                if shortcuts::is_delete(event) {
                    self.delete();
                    return EventResponse::handled();
                }

                // Arrow keys
                match &event.key {
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.move_cursor_left(event.modifiers.shift());
                        return EventResponse::handled();
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        self.move_cursor_right(event.modifiers.shift());
                        return EventResponse::handled();
                    }
                    Key::Named(NamedKey::Home) => {
                        if !event.modifiers.shift() {
                            self.selection_start = None;
                        } else if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_pos);
                        }
                        self.cursor_pos = 0;
                        return EventResponse::handled();
                    }
                    Key::Named(NamedKey::End) => {
                        if !event.modifiers.shift() {
                            self.selection_start = None;
                        } else if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_pos);
                        }
                        self.cursor_pos = self.value.len();
                        return EventResponse::handled();
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(handler) = &mut self.on_submit {
                            handler(&self.value);
                        }
                        return EventResponse::handled();
                    }
                    Key::Named(NamedKey::Escape) => {
                        ctx.release_focus();
                        return EventResponse {
                            release_focus: true,
                            repaint: true,
                            ..Default::default()
                        };
                    }
                    _ => {}
                }

                EventResponse::default()
            }
            InputEvent::TextInput { text } => {
                if ctx.has_focus() {
                    // Filter out control characters
                    for c in text.chars() {
                        if !c.is_control() {
                            self.insert_char(c);
                        }
                    }
                    return EventResponse::handled();
                }
                EventResponse::default()
            }
            _ => EventResponse::default(),
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let text_style = TextStyle::default().with_size(self.style.font_size);
        self.update_prefix_width_cache_with_layout_ctx(ctx, &text_style);
        let sample = if self.value.is_empty() {
            if self.placeholder.is_empty() {
                "M"
            } else {
                &self.placeholder
            }
        } else {
            &self.value
        };
        let (text_width, text_height) = ctx.measure_text(sample, &text_style);

        let width = (text_width + self.style.padding_h * 2.0 + self.style.border_width * 2.0)
            .max(self.style.min_width);
        let height = (text_height + self.style.padding_v * 2.0 + self.style.border_width * 2.0)
            .max(self.style.min_height);
        Some((width, height))
    }

    fn on_focus(&mut self) {
        // Select all on focus
        self.select_all();
    }

    fn on_blur(&mut self) {
        self.selection_start = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{layout_bounds, mock_event_context, pointer_down_at};
    use spark_input::FocusManager;
    use spark_layout::LayoutTree;
    use spark_text::TextSystem;

    fn prepare_input_with_cache(input: &TextInput) {
        let mut text = TextSystem::new_headless();
        let mut ctx = crate::LayoutContext {
            text: &mut text,
            max_width: None,
            max_height: None,
        };
        let _ = input.measure(&mut ctx);
    }

    #[test]
    fn pointer_click_places_cursor_at_start_middle_end() {
        let mut input = TextInput::new().value("hello");
        input.set_id(Default::default());
        prepare_input_with_cache(&input);

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        let mut event_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);

        let _ = input.event(&mut event_ctx, &pointer_down_at(2.0, 18.0));
        assert_eq!(input.cursor_pos, 0);

        let mid_prefix_width = input
            .prefix_widths
            .borrow()
            .iter()
            .find_map(|(idx, width)| (*idx == 3).then_some(*width))
            .expect("missing width for index 3");
        let _ = input.event(
            &mut event_ctx,
            &pointer_down_at(mid_prefix_width + input.style.padding_h, 18.0),
        );
        assert_eq!(input.cursor_pos, 3);

        let _ = input.event(&mut event_ctx, &pointer_down_at(238.0, 18.0));
        assert_eq!(input.cursor_pos, input.value.len());
    }
}
