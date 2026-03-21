//! Multiline text area widget.

use crate::text_editor::{EditorCore, TextEditorState};
use crate::text_input::TextInputStyle;
use crate::{
    control_state::{focus_ring_border_width, focus_ring_bounds, focus_ring_color},
    current_theme, AccessibilityAction, AccessibilityInfo, AccessibilityRole, EventContext,
    PaintContext, Widget,
};
use sparsh_core::Color;
use sparsh_input::{Action, ActionMapper, InputEvent, Key, NamedKey, StandardAction};
use sparsh_layout::WidgetId;
use sparsh_text::TextStyle;
use std::cell::RefCell;
use taffy::prelude::*;

type TextAreaCallback = Box<dyn FnMut(&str)>;
pub type TextAreaStyle = TextInputStyle;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static TEXT_MEASURE_SPAN: RefCell<Option<web_sys::Element>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
struct LineMetrics {
    start: usize,
    end: usize,
    widths: Vec<(usize, f32)>,
}

/// A multiline text editor widget.
pub struct TextArea {
    id: WidgetId,
    editor: EditorCore,
    placeholder: String,
    style: TextAreaStyle,
    on_change: Option<TextAreaCallback>,
    fill_width: bool,
    use_theme_defaults: bool,
    line_metrics: RefCell<Vec<LineMetrics>>,
}

impl TextArea {
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            editor: EditorCore::new(String::new()),
            placeholder: String::new(),
            style: TextAreaStyle {
                min_height: 96.0,
                ..TextAreaStyle::default()
            },
            on_change: None,
            fill_width: false,
            use_theme_defaults: true,
            line_metrics: RefCell::new(vec![LineMetrics {
                start: 0,
                end: 0,
                widths: vec![(0, 0.0)],
            }]),
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.editor.set_text(value.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn on_change(mut self, handler: impl FnMut(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    pub fn with_style(mut self, style: TextAreaStyle) -> Self {
        self.style = style;
        self.use_theme_defaults = false;
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.fill_width = true;
        self
    }

    pub fn get_value(&self) -> &str {
        self.editor.text()
    }

    fn fire_change(&mut self) {
        if let Some(handler) = &mut self.on_change {
            handler(self.editor.text());
        }
    }

    fn resolved_style(&self) -> TextAreaStyle {
        if self.use_theme_defaults {
            let theme = current_theme();
            TextAreaStyle {
                background: theme.colors.input_background,
                background_focused: theme.colors.surface,
                text_color: theme.colors.text_primary,
                placeholder_color: theme.colors.input_placeholder,
                border_color: theme.colors.border,
                border_color_focused: theme.colors.primary,
                border_width: 1.0,
                corner_radius: theme.radii.md,
                padding_h: theme.controls.control_padding_x,
                padding_v: theme.controls.control_padding_y,
                font_size: theme.typography.body_size,
                min_width: 180.0,
                min_height: 96.0,
            }
        } else {
            self.style.clone()
        }
    }

    fn compute_prefix_widths(
        text: &str,
        start: usize,
        mut measure_width: impl FnMut(&str) -> f32,
    ) -> Vec<(usize, f32)> {
        let mut boundaries = vec![(start, 0.0)];
        if text.is_empty() {
            return boundaries;
        }
        for (offset, _) in text.char_indices().skip(1) {
            boundaries.push((start + offset, measure_width(&text[..offset])));
        }
        boundaries.push((start + text.len(), measure_width(text)));
        boundaries
    }

    fn update_line_metrics_with_paint_ctx(&self, ctx: &mut PaintContext, style: &TextStyle) {
        let lines = line_slices(self.editor.text());
        let metrics = lines
            .iter()
            .map(|(start, end, slice)| LineMetrics {
                start: *start,
                end: *end,
                widths: Self::compute_prefix_widths(slice, *start, |prefix| {
                    self.measure_prefix_width(ctx, style, prefix) / ctx.scale_factor.max(1.0)
                }),
            })
            .collect();
        *self.line_metrics.borrow_mut() = metrics;
    }

    fn update_line_metrics_with_layout_ctx(
        &self,
        ctx: &mut crate::LayoutContext,
        style: &TextStyle,
    ) {
        let lines = line_slices(self.editor.text());
        let metrics = lines
            .iter()
            .map(|(start, end, slice)| LineMetrics {
                start: *start,
                end: *end,
                widths: Self::compute_prefix_widths(slice, *start, |prefix| {
                    ctx.measure_text(prefix, style).0
                }),
            })
            .collect();
        *self.line_metrics.borrow_mut() = metrics;
    }

    fn measure_prefix_width(&self, ctx: &mut PaintContext, style: &TextStyle, text: &str) -> f32 {
        #[cfg(target_arch = "wasm32")]
        if let Some(width) = measure_text_width_dom(text, style, ctx.scale_factor) {
            return width;
        }

        ctx.measure_text(text, style).0
    }

    fn cursor_index_for_position(&self, x: f32, y: f32, line_height: f32) -> usize {
        let metrics = self.line_metrics.borrow();
        if metrics.is_empty() {
            return 0;
        }
        let line = (y / line_height).floor().max(0.0) as usize;
        let Some(line_metrics) = metrics.get(line).or_else(|| metrics.last()) else {
            return 0;
        };

        if x <= 0.0 {
            return line_metrics.start;
        }
        if let Some((last_idx, last_x)) = line_metrics.widths.last() {
            if x >= *last_x {
                return *last_idx;
            }
        }

        let mut best = (line_metrics.end, f32::MAX);
        for (idx, width) in &line_metrics.widths {
            let distance = (*width - x).abs();
            if distance < best.1 {
                best = (*idx, distance);
            }
        }
        best.0
    }

    fn handle_action(&mut self, ctx: &mut EventContext, action: StandardAction) {
        match action {
            StandardAction::SelectAll => {
                self.editor.select_all();
                ctx.request_paint();
            }
            StandardAction::Copy => {
                if let Some(text) = self.editor.copy_selection() {
                    ctx.write_clipboard(text);
                }
                ctx.request_paint();
            }
            StandardAction::Cut => {
                if let Some(text) = self.editor.cut_selection() {
                    ctx.write_clipboard(text);
                    self.fire_change();
                    ctx.request_layout();
                }
            }
            StandardAction::Undo if self.editor.undo() => {
                self.fire_change();
                ctx.request_layout();
            }
            StandardAction::Redo if self.editor.redo() => {
                self.fire_change();
                ctx.request_layout();
            }
            StandardAction::Backspace if self.editor.backspace() => {
                self.fire_change();
                ctx.request_layout();
            }
            StandardAction::Delete if self.editor.delete_forward() => {
                self.fire_change();
                ctx.request_layout();
            }
            StandardAction::MoveLeft => {
                self.editor.move_left(false);
                ctx.request_paint();
            }
            StandardAction::MoveRight => {
                self.editor.move_right(false);
                ctx.request_paint();
            }
            StandardAction::MoveUp => {
                self.editor.move_up(false);
                ctx.request_paint();
            }
            StandardAction::MoveDown => {
                self.editor.move_down(false);
                ctx.request_paint();
            }
            StandardAction::SelectLeft => {
                self.editor.move_left(true);
                ctx.request_paint();
            }
            StandardAction::SelectRight => {
                self.editor.move_right(true);
                ctx.request_paint();
            }
            StandardAction::SelectUp => {
                self.editor.move_up(true);
                ctx.request_paint();
            }
            StandardAction::SelectDown => {
                self.editor.move_down(true);
                ctx.request_paint();
            }
            StandardAction::MoveWordLeft => {
                self.editor.move_word_left(false);
                ctx.request_paint();
            }
            StandardAction::MoveWordRight => {
                self.editor.move_word_right(false);
                ctx.request_paint();
            }
            StandardAction::SelectWordLeft => {
                self.editor.move_word_left(true);
                ctx.request_paint();
            }
            StandardAction::SelectWordRight => {
                self.editor.move_word_right(true);
                ctx.request_paint();
            }
            StandardAction::MoveToStart => {
                self.editor.move_to_start(false);
                ctx.request_paint();
            }
            StandardAction::MoveToEnd => {
                self.editor.move_to_end(false);
                ctx.request_paint();
            }
            StandardAction::SelectToStart => {
                self.editor.move_to_start(true);
                ctx.request_paint();
            }
            StandardAction::SelectToEnd => {
                self.editor.move_to_end(true);
                ctx.request_paint();
            }
            _ => {}
        }
        ctx.stop_propagation();
    }
}

#[cfg(target_arch = "wasm32")]
fn measure_text_width_dom(text: &str, style: &TextStyle, scale_factor: f32) -> Option<f32> {
    let document = web_sys::window()?.document()?;
    let span = TEXT_MEASURE_SPAN.with(|slot| {
        if let Some(existing) = slot.borrow().as_ref() {
            return Some(existing.clone());
        }

        let body = document.body()?;
        let node = document.create_element("span").ok()?;
        body.append_child(&node).ok()?;
        *slot.borrow_mut() = Some(node.clone());
        Some(node)
    })?;

    let family = if style.family.trim().is_empty() || style.family == "system-ui" {
        "sans-serif"
    } else {
        style.family.as_str()
    };

    let css = format!(
        "position:absolute;visibility:hidden;white-space:pre;left:-100000px;top:-100000px;\
         pointer-events:none;font-size:{}px;font-family:{};font-style:{};font-weight:{};\
         line-height:{};",
        style.font_size * scale_factor,
        family,
        if style.italic { "italic" } else { "normal" },
        if style.bold { "700" } else { "400" },
        style.line_height
    );
    span.set_attribute("style", &css).ok()?;
    span.set_text_content(Some(text));
    Some(span.get_bounding_client_rect().width() as f32)
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TextArea {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        let style = self.resolved_style();
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
                left: length(style.padding_h),
                right: length(style.padding_h),
                top: length(style.padding_v),
                bottom: length(style.padding_v),
            },
            min_size: Size {
                width: length(style.min_width),
                height: length(style.min_height),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let style = self.resolved_style();
        let bounds = ctx.bounds();
        let focused = ctx.has_focus();
        let scale = ctx.scale_factor;

        let bg = if focused {
            style.background_focused
        } else {
            style.background
        };
        let border = if focused {
            style.border_color_focused
        } else {
            style.border_color
        };
        ctx.fill_bordered_rect(bounds, bg, style.corner_radius, style.border_width, border);

        if focused {
            let controls = current_theme().controls;
            let focus_bounds = focus_ring_bounds(bounds, scale, &controls);
            ctx.fill_bordered_rect(
                focus_bounds,
                Color::TRANSPARENT,
                style.corner_radius + 2.0,
                focus_ring_border_width(scale, &controls),
                focus_ring_color(current_theme().colors.border_focus),
            );
        }

        let padding_h = style.padding_h * scale;
        let padding_v = style.padding_v * scale;
        let text_x = bounds.x + padding_h;
        let text_y = bounds.y + padding_v;
        let text_style = TextStyle::default()
            .with_family(current_theme().typography.font_family.clone())
            .with_size(style.font_size)
            .with_color(style.text_color);
        let placeholder_style = TextStyle::default()
            .with_family(current_theme().typography.font_family)
            .with_size(style.font_size)
            .with_color(style.placeholder_color);

        self.update_line_metrics_with_paint_ctx(ctx, &text_style);
        let (_, line_height) = ctx.measure_text("Ay", &text_style);
        let lines = line_slices(self.editor.text());

        if self.editor.text().is_empty() {
            if !self.placeholder.is_empty() {
                ctx.draw_text(&self.placeholder, &placeholder_style, text_x, text_y);
            }
        } else {
            if let Some((selection_start, selection_end)) = self.editor.selection_range() {
                for (line_index, (start, end, _slice)) in lines.iter().enumerate() {
                    let highlight_start = selection_start.max(*start);
                    let highlight_end = selection_end.min(*end);
                    if highlight_start >= highlight_end {
                        continue;
                    }

                    let line_metrics = self
                        .line_metrics
                        .borrow()
                        .get(line_index)
                        .cloned()
                        .unwrap_or(LineMetrics {
                            start: *start,
                            end: *end,
                            widths: vec![(*start, 0.0)],
                        });

                    let start_x = line_metrics
                        .widths
                        .iter()
                        .find_map(|(idx, width)| (*idx == highlight_start).then_some(*width))
                        .unwrap_or_default()
                        * scale;
                    let end_x = line_metrics
                        .widths
                        .iter()
                        .find_map(|(idx, width)| (*idx == highlight_end).then_some(*width))
                        .unwrap_or(start_x / scale)
                        * scale;

                    ctx.fill_rect(
                        sparsh_core::Rect::new(
                            text_x + start_x,
                            text_y + line_index as f32 * line_height,
                            (end_x - start_x).max(0.0),
                            line_height,
                        ),
                        Color::from_hex(0x3B82F6).with_alpha(0.3),
                    );
                }
            }

            for (line_index, (_start, _end, slice)) in lines.iter().enumerate() {
                ctx.draw_text(
                    slice,
                    &text_style,
                    text_x,
                    text_y + line_index as f32 * line_height,
                );
            }
        }

        if focused {
            ctx.request_next_frame();
            let cursor_visible = (ctx.elapsed_time * 2.0).fract() < 0.5;
            if cursor_visible {
                let (line, _column) = self.editor.line_and_column(self.editor.cursor());
                let line_metrics =
                    self.line_metrics
                        .borrow()
                        .get(line)
                        .cloned()
                        .unwrap_or(LineMetrics {
                            start: 0,
                            end: 0,
                            widths: vec![(0, 0.0)],
                        });
                let cursor_x = line_metrics
                    .widths
                    .iter()
                    .find_map(|(idx, width)| (*idx == self.editor.cursor()).then_some(*width))
                    .unwrap_or_default()
                    * scale;
                ctx.fill_rect(
                    sparsh_core::Rect::new(
                        text_x + cursor_x,
                        text_y + line as f32 * line_height,
                        2.0 * scale,
                        line_height,
                    ),
                    style.text_color,
                );
            }
        }
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        match event {
            InputEvent::PointerDown { pos, .. } if ctx.contains(*pos) => {
                let style = self.resolved_style();
                ctx.request_focus();
                let local = ctx.to_local(*pos);
                let line_height = style.font_size * 1.2;
                let x = (local.x - style.padding_h).max(0.0);
                let y = (local.y - style.padding_v).max(0.0);
                let index = self.cursor_index_for_position(x, y, line_height);
                self.editor.set_cursor(index, false);
                ctx.capture_pointer();
            }
            InputEvent::PointerMove { pos } if ctx.has_capture => {
                let style = self.resolved_style();
                let local = ctx.to_local(*pos);
                let line_height = style.font_size * 1.2;
                let x = (local.x - style.padding_h).max(0.0);
                let y = (local.y - style.padding_v).max(0.0);
                let index = self.cursor_index_for_position(x, y, line_height);
                self.editor.set_cursor(index, true);
                ctx.request_paint();
            }
            InputEvent::PointerUp { pos, .. } if ctx.has_capture => {
                let style = self.resolved_style();
                let local = ctx.to_local(*pos);
                let line_height = style.font_size * 1.2;
                let x = (local.x - style.padding_h).max(0.0);
                let y = (local.y - style.padding_v).max(0.0);
                let index = self.cursor_index_for_position(x, y, line_height);
                self.editor.set_cursor(index, true);
                ctx.release_pointer();
            }
            InputEvent::KeyDown { event } => {
                if !ctx.has_focus() {
                    return;
                }

                match &event.key {
                    Key::Named(NamedKey::Enter) => {
                        if self.editor.insert_text("\n", true) {
                            self.fire_change();
                            ctx.stop_propagation();
                            ctx.request_layout();
                        }
                        return;
                    }
                    Key::Named(NamedKey::Escape) => {
                        self.editor.clear_composition();
                        ctx.release_focus();
                        ctx.stop_propagation();
                        ctx.request_paint();
                        return;
                    }
                    _ => {}
                }

                let mapper = ActionMapper::new();
                let input_event = InputEvent::KeyDown {
                    event: event.clone(),
                };
                if let Some(Action::Standard(action)) = mapper.map_event(&input_event) {
                    self.handle_action(ctx, action);
                }
            }
            InputEvent::TextInput { text }
                if ctx.has_focus() && self.editor.insert_text(text, true) =>
            {
                self.fire_change();
                ctx.stop_propagation();
                ctx.request_layout();
            }
            InputEvent::Paste { text } if ctx.has_focus() && self.editor.paste_text(text, true) => {
                self.fire_change();
                ctx.stop_propagation();
                ctx.request_layout();
            }
            InputEvent::CompositionStart if ctx.has_focus() => {
                self.editor.begin_composition();
                ctx.stop_propagation();
                ctx.request_paint();
            }
            InputEvent::CompositionUpdate { text }
                if ctx.has_focus() && self.editor.update_composition(text, true) =>
            {
                ctx.stop_propagation();
                ctx.request_layout();
            }
            InputEvent::CompositionEnd { text }
                if ctx.has_focus() && self.editor.end_composition(text, true) =>
            {
                self.fire_change();
                ctx.stop_propagation();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let style = self.resolved_style();
        let text_style = TextStyle::default()
            .with_family(current_theme().typography.font_family)
            .with_size(style.font_size);
        self.update_line_metrics_with_layout_ctx(ctx, &text_style);

        let sample = if self.editor.text().is_empty() {
            if self.placeholder.is_empty() {
                "M"
            } else {
                &self.placeholder
            }
        } else {
            self.editor.text()
        };
        let lines = line_slices(sample);
        let mut max_width: f32 = 0.0;
        let mut line_height: f32 = 0.0;
        for (_, _, slice) in &lines {
            let (width, height) = ctx.measure_text(slice, &text_style);
            max_width = max_width.max(width);
            line_height = line_height.max(height);
        }
        if line_height <= 0.0 {
            line_height = ctx.measure_text("Ay", &text_style).1;
        }

        let width =
            (max_width + style.padding_h * 2.0 + style.border_width * 2.0).max(style.min_width);
        let height =
            (line_height * lines.len() as f32 + style.padding_v * 2.0 + style.border_width * 2.0)
                .max(style.min_height);
        Some((width, height))
    }

    fn on_focus(&mut self) {
        self.editor.clear_composition();
    }

    fn on_blur(&mut self) {
        self.editor.clear_selection();
        self.editor.clear_composition();
    }

    fn text_editor_state(&self) -> Option<TextEditorState> {
        Some(self.editor.state(true))
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        let mut info = AccessibilityInfo::new(AccessibilityRole::MultilineTextInput)
            .value(self.editor.text().to_owned())
            .action(AccessibilityAction::Focus)
            .action(AccessibilityAction::SetValue);
        if !self.placeholder.is_empty() {
            info.label = Some(self.placeholder.clone());
        }
        Some(info)
    }

    fn handle_accessibility_action(
        &mut self,
        action: AccessibilityAction,
        value: Option<String>,
    ) -> bool {
        if matches!(action, AccessibilityAction::SetValue) {
            let next = value.unwrap_or_default();
            if self.editor.text() != next {
                self.editor.set_text(next);
                self.fire_change();
                return true;
            }
        }
        false
    }
}

fn line_slices(text: &str) -> Vec<(usize, usize, &str)> {
    let mut lines = Vec::new();
    if text.is_empty() {
        lines.push((0, 0, ""));
        return lines;
    }
    let mut start = 0usize;
    for segment in text.split('\n') {
        let end = start + segment.len();
        lines.push((start, end, segment));
        start = end.saturating_add(1);
    }
    if text.ends_with('\n') {
        lines.push((text.len(), text.len(), ""));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{layout_bounds, mock_event_context};
    use crate::{PaintCommands, PaintContext};
    use sparsh_input::{FocusManager, InputEvent, KeyboardEvent, Modifiers};
    use sparsh_layout::LayoutTree;
    use sparsh_render::DrawList;
    use sparsh_text::TextSystem;

    fn primary_modifiers() -> Modifiers {
        #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
        {
            Modifiers::META
        }

        #[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
        {
            Modifiers::CONTROL
        }
    }

    #[test]
    fn enter_inserts_newline_and_vertical_navigation_moves_cursor() {
        let mut area = TextArea::new().value("hello");
        area.set_id(Default::default());
        let layout = layout_bounds(0.0, 0.0, 280.0, 120.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(area.id());

        let mut enter_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
        area.event(
            &mut enter_ctx,
            &InputEvent::KeyDown {
                event: KeyboardEvent::key_down(Key::Named(NamedKey::Enter), Default::default()),
            },
        );
        assert_eq!(area.get_value(), "hello\n");

        area.event(
            &mut enter_ctx,
            &InputEvent::TextInput {
                text: "world".to_owned(),
            },
        );
        assert_eq!(area.get_value(), "hello\nworld");

        area.editor.move_to_end(false);
        let mut up_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
        area.event(
            &mut up_ctx,
            &InputEvent::KeyDown {
                event: KeyboardEvent::key_down(Key::Named(NamedKey::ArrowUp), Default::default()),
            },
        );
        assert_eq!(area.editor.line_and_column(area.editor.cursor()).0, 0);
    }

    #[test]
    fn paste_and_undo_work_for_multiline_content() {
        let mut area = TextArea::new();
        area.set_id(Default::default());
        let layout = layout_bounds(0.0, 0.0, 280.0, 120.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(area.id());

        let mut paste_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
        area.event(
            &mut paste_ctx,
            &InputEvent::Paste {
                text: "alpha\nbeta".to_owned(),
            },
        );
        assert_eq!(area.get_value(), "alpha\nbeta");

        let mut undo_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
        area.event(
            &mut undo_ctx,
            &InputEvent::KeyDown {
                event: KeyboardEvent {
                    key: Key::Character("z".into()),
                    modifiers: primary_modifiers(),
                    ..Default::default()
                },
            },
        );
        assert_eq!(area.get_value(), "");
    }

    #[test]
    fn pointer_hit_testing_stays_correct_after_scaled_paint() {
        let mut area = TextArea::new().value("line one\nline two");
        area.set_id(Default::default());

        let layout_tree = LayoutTree::new();
        let focus = FocusManager::new();
        let mut draw_list = DrawList::new();
        let mut text = TextSystem::new_headless();
        let mut commands = PaintCommands::default();
        let mut paint_ctx = PaintContext {
            draw_list: &mut draw_list,
            layout: layout_bounds(0.0, 0.0, 560.0, 240.0),
            layout_tree: &layout_tree,
            focus: &focus,
            widget_id: area.id(),
            scale_factor: 2.0,
            text_system: &mut text,
            elapsed_time: 0.0,
            commands: &mut commands,
        };
        area.paint(&mut paint_ctx);

        let layout = layout_bounds(0.0, 0.0, 280.0, 120.0);
        let mut event_focus = FocusManager::new();
        let mut event_ctx =
            mock_event_context(layout, &layout_tree, &mut event_focus, area.id(), false);
        area.event(
            &mut event_ctx,
            &InputEvent::PointerDown {
                pos: glam::vec2(278.0, 40.0),
                button: sparsh_input::PointerButton::Primary,
            },
        );

        assert_eq!(area.editor.cursor(), area.get_value().len());
    }
}
