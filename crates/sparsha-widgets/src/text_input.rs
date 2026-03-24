//! Text input widget.

use crate::text_editor::{EditorCore, TextEditorState};
use crate::text_editor_widget::{
    apply_standard_action, compute_prefix_widths, editor_placeholder_style, editor_text_style,
    editor_widget_style, paint_editor_frame, persist_editor_build_state, resolve_editor_style,
    restore_editor_build_state, set_editor_value,
};
use crate::{
    AccessibilityAction, AccessibilityInfo, AccessibilityRole, EventContext, PaintContext, Widget,
};
use bon::bon;
use sparsha_core::Color;
use sparsha_input::{Action, ActionMapper, InputEvent, Key, NamedKey, StandardAction};
use sparsha_layout::WidgetId;
use sparsha_text::{TextLayoutInfo, TextLayoutOptions, TextStyle, TextWrap};
use std::cell::RefCell;
use taffy::prelude::*;

/// Callback type for text change and submit handlers.
type TextInputCallback = Box<dyn FnMut(&str)>;

#[derive(Clone, Debug)]
struct LineMetrics {
    x_offset: f32,
    y_offset: f32,
    height: f32,
    widths: Vec<(usize, f32)>,
}

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
            min_height: 38.0,
        }
    }
}

/// A single-line text input widget.
pub struct TextInput {
    id: WidgetId,
    editor: EditorCore,
    declared_value: Option<String>,
    placeholder: String,
    style: TextInputStyle,
    on_change: Option<TextInputCallback>,
    on_submit: Option<TextInputCallback>,
    fill_width: bool,
    use_theme_defaults: bool,
    line_metrics: RefCell<LineMetrics>,
}

impl TextInput {
    fn empty() -> Self {
        Self {
            id: WidgetId::default(),
            editor: EditorCore::new(String::new()),
            declared_value: None,
            placeholder: String::new(),
            style: TextInputStyle::default(),
            on_change: None,
            on_submit: None,
            fill_width: false,
            use_theme_defaults: true,
            line_metrics: RefCell::new(LineMetrics {
                x_offset: 0.0,
                y_offset: 0.0,
                height: 0.0,
                widths: vec![(0, 0.0)],
            }),
        }
    }

    fn style_override(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self.use_theme_defaults = false;
        self
    }

    /// Get the current value.
    pub fn get_value(&self) -> &str {
        self.editor.text()
    }

    fn fire_change(&mut self) {
        if let Some(handler) = &mut self.on_change {
            handler(self.editor.text());
        }
    }

    fn update_line_metrics(
        &self,
        layout_info: &TextLayoutInfo,
        style: &TextStyle,
        mut measure_prefix: impl FnMut(&str) -> f32,
    ) {
        if self.editor.text().is_empty() {
            *self.line_metrics.borrow_mut() = LineMetrics {
                x_offset: 0.0,
                y_offset: 0.0,
                height: style.font_size * style.line_height,
                widths: vec![(0, 0.0)],
            };
            return;
        }

        let Some(line) = layout_info.lines.first().cloned() else {
            *self.line_metrics.borrow_mut() = LineMetrics {
                x_offset: 0.0,
                y_offset: 0.0,
                height: style.font_size * style.line_height,
                widths: vec![(0, 0.0)],
            };
            return;
        };
        let widths = compute_prefix_widths(self.editor.text(), 0, |prefix| measure_prefix(prefix));
        *self.line_metrics.borrow_mut() = LineMetrics {
            x_offset: line.offset,
            y_offset: line.min_coord,
            height: line.line_height,
            widths,
        };
    }

    fn update_line_metrics_with_paint_ctx(&self, ctx: &mut PaintContext, style: &TextStyle) {
        let layout = ctx.text_system.layout_info(
            self.editor.text(),
            style,
            TextLayoutOptions::new()
                .with_wrap(TextWrap::NoWrap)
                .with_max_lines(Some(1)),
        );
        self.update_line_metrics(&layout, style, |prefix| {
            ctx.text_system.measure(prefix, style, None).0
        });
    }

    fn update_line_metrics_with_layout_ctx(
        &self,
        ctx: &mut crate::LayoutContext,
        style: &TextStyle,
    ) {
        let layout = ctx.text.layout_info(
            self.editor.text(),
            style,
            TextLayoutOptions::new()
                .with_wrap(TextWrap::NoWrap)
                .with_max_lines(Some(1)),
        );
        self.update_line_metrics(&layout, style, |prefix| {
            ctx.text.measure(prefix, style, None).0
        });
    }

    fn cursor_index_for_x(&self, x: f32) -> usize {
        if self.editor.text().is_empty() {
            return 0;
        }
        let line = self.line_metrics.borrow();
        if line.widths.is_empty() {
            return self.editor.text().len();
        }

        if x <= line.x_offset {
            return 0;
        }
        let local_x = (x - line.x_offset).max(0.0);
        if let Some((last_idx, last_x)) = line.widths.last() {
            if local_x >= *last_x {
                return *last_idx;
            }
        }

        let mut best = (self.editor.text().len(), f32::MAX);
        for (idx, width) in &line.widths {
            let dist = (*width - local_x).abs();
            if dist < best.1 {
                best = (*idx, dist);
            }
        }
        best.0
    }

    fn prefix_width_for(&self, index: usize) -> Option<f32> {
        self.line_metrics
            .borrow()
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == index).then_some(*width))
    }

    fn range_width_for(&self, start: usize, end: usize) -> f32 {
        let start_x = self.prefix_width_for(start).unwrap_or_default();
        let end_x = self.prefix_width_for(end).unwrap_or(start_x);
        (end_x - start_x).max(0.0)
    }

    fn cursor_offset_for(&self, ctx: &mut PaintContext, style: &TextStyle) -> f32 {
        if let Some(width) = self.prefix_width_for(self.editor.cursor()) {
            return (self.line_metrics.borrow().x_offset + width) * ctx.scale_factor;
        }
        let text_before_cursor = &self.editor.text()[..self.editor.cursor()];
        self.line_metrics.borrow().x_offset * ctx.scale_factor
            + ctx.measure_text(text_before_cursor, style).0
    }

    fn resolved_style(&self) -> TextInputStyle {
        resolve_editor_style(&self.style, self.use_theme_defaults, false)
    }

    fn handle_action(&mut self, ctx: &mut EventContext, action: StandardAction) -> bool {
        let changed = apply_standard_action(&mut self.editor, ctx, action, false);
        if changed {
            self.fire_change();
        }
        changed
    }
}

#[bon]
impl TextInput {
    #[builder(
        start_fn(name = builder, vis = "pub"),
        finish_fn(name = build, vis = "pub"),
        builder_type(name = TextInputBuilder, vis = "pub"),
        state_mod(vis = "pub")
    )]
    fn builder_init(
        #[builder(with = |value: impl Into<String>| value.into())] value: Option<String>,
        #[builder(into, default = String::new())] placeholder: String,
        style: Option<TextInputStyle>,
        #[builder(default)] fill_width: bool,
        #[builder(with = |handler: impl FnMut(&str) + 'static| Box::new(handler) as TextInputCallback)]
        on_change: Option<TextInputCallback>,
        #[builder(with = |handler: impl FnMut(&str) + 'static| Box::new(handler) as TextInputCallback)]
        on_submit: Option<TextInputCallback>,
    ) -> Self {
        let mut input = Self::empty();
        if let Some(value) = value {
            input.editor.set_text(value.clone());
            input.declared_value = Some(value);
        }
        if !placeholder.is_empty() {
            input.placeholder = placeholder;
        }
        if let Some(style) = style {
            input = input.style_override(style);
        }
        if fill_width {
            input.fill_width = true;
        }
        if let Some(on_change) = on_change {
            input.on_change = Some(on_change);
        }
        if let Some(on_submit) = on_submit {
            input.on_submit = Some(on_submit);
        }
        input
    }
}

impl Widget for TextInput {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn rebuild(&mut self, ctx: &mut crate::BuildContext) {
        restore_editor_build_state(&mut self.editor, &self.declared_value, false, ctx);
        persist_editor_build_state(&self.editor, &self.declared_value, false, ctx);
    }

    fn persist_build_state(&self, ctx: &mut crate::BuildContext) {
        persist_editor_build_state(&self.editor, &self.declared_value, false, ctx);
    }

    fn style(&self) -> Style {
        editor_widget_style(&self.resolved_style(), self.fill_width)
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let style = self.resolved_style();
        let bounds = ctx.bounds();
        let scale = ctx.scale_factor;
        let focused = ctx.has_focus();
        paint_editor_frame(ctx, bounds, &style);

        let padding_h = style.padding_h * scale;
        let text_x = bounds.x + padding_h;
        let text_width = bounds.width - padding_h * 2.0;

        let text_style = editor_text_style(&style);
        let placeholder_style = editor_placeholder_style(&style);

        self.update_line_metrics_with_paint_ctx(ctx, &text_style);

        let line_metrics = self.line_metrics.borrow().clone();
        let text_height = line_metrics
            .height
            .max(text_style.font_size * text_style.line_height)
            * scale;
        let text_y = bounds.y + (bounds.height - text_height) / 2.0;

        if self.editor.text().is_empty() {
            if !self.placeholder.is_empty() {
                ctx.draw_text(&self.placeholder, &placeholder_style, text_x, text_y);
            }
        } else {
            if let Some((start, end)) = self.editor.selection_range() {
                let sel_x_start = self.prefix_width_for(start).unwrap_or_default();
                let sel_width = self.range_width_for(start, end);
                if sel_width > 0.0 {
                    let sel_rect = sparsha_core::Rect::new(
                        text_x + (line_metrics.x_offset + sel_x_start) * scale,
                        text_y + line_metrics.y_offset * scale,
                        (sel_width.min(text_width / scale - sel_x_start)).max(0.0) * scale,
                        text_height,
                    );
                    ctx.fill_rect(sel_rect, Color::from_hex(0x3B82F6).with_alpha(0.3));
                }
            }

            ctx.draw_text(
                self.editor.text(),
                &text_style,
                text_x + line_metrics.x_offset * scale,
                text_y,
            );
        }

        if focused {
            ctx.request_next_frame();
            let cursor_visible = (ctx.elapsed_time * 2.0).fract() < 0.5;
            if cursor_visible {
                let cursor_x_offset = self.cursor_offset_for(ctx, &text_style);
                let cursor_x = text_x + cursor_x_offset;
                let cursor_rect =
                    sparsha_core::Rect::new(cursor_x, text_y, 2.0 * scale, text_height);
                ctx.fill_rect(cursor_rect, style.text_color);
            }
        }
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        match event {
            InputEvent::PointerDown { pos, .. } if ctx.contains(*pos) => {
                let style = self.resolved_style();
                ctx.request_focus();
                let local = ctx.to_local(*pos);
                let click_x = (local.x - style.padding_h).max(0.0);
                self.editor
                    .set_cursor(self.cursor_index_for_x(click_x), false);
                ctx.capture_pointer();
            }
            InputEvent::PointerMove { pos } if ctx.has_capture => {
                let style = self.resolved_style();
                let local = ctx.to_local(*pos);
                let click_x = (local.x - style.padding_h).max(0.0);
                self.editor
                    .set_cursor(self.cursor_index_for_x(click_x), true);
                ctx.request_paint();
            }
            InputEvent::PointerUp { pos, .. } if ctx.has_capture => {
                let style = self.resolved_style();
                let local = ctx.to_local(*pos);
                let click_x = (local.x - style.padding_h).max(0.0);
                self.editor
                    .set_cursor(self.cursor_index_for_x(click_x), true);
                ctx.release_pointer();
            }
            InputEvent::KeyDown { event } => {
                if !ctx.has_focus() {
                    return;
                }

                match &event.key {
                    Key::Named(NamedKey::Enter) => {
                        if let Some(handler) = &mut self.on_submit {
                            handler(self.editor.text());
                        }
                        ctx.stop_propagation();
                        ctx.request_paint();
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
                    let _ = self.handle_action(ctx, action);
                }
            }
            InputEvent::TextInput { text }
                if ctx.has_focus() && self.editor.insert_text(text, false) =>
            {
                self.fire_change();
                ctx.stop_propagation();
                ctx.request_layout();
            }
            InputEvent::Paste { text }
                if ctx.has_focus() && self.editor.paste_text(text, false) =>
            {
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
                if ctx.has_focus() && self.editor.update_composition(text, false) =>
            {
                ctx.stop_propagation();
                ctx.request_layout();
            }
            InputEvent::CompositionEnd { text }
                if ctx.has_focus() && self.editor.end_composition(text, false) =>
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
        let text_style = editor_text_style(&style);
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
        let layout = ctx.text.layout_info(
            sample,
            &text_style,
            TextLayoutOptions::new()
                .with_wrap(TextWrap::NoWrap)
                .with_max_lines(Some(1)),
        );
        let text_width = layout.width;
        let text_height = layout
            .height
            .max(text_style.font_size * text_style.line_height);

        let width =
            (text_width + style.padding_h * 2.0 + style.border_width * 2.0).max(style.min_width);
        let height =
            (text_height + style.padding_v * 2.0 + style.border_width * 2.0).max(style.min_height);
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
        Some(self.editor.state(false))
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        let mut info = AccessibilityInfo::new(AccessibilityRole::TextInput)
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
        if matches!(action, AccessibilityAction::SetValue)
            && set_editor_value(&mut self.editor, value)
        {
            self.fire_change();
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        layout_bounds, mock_event_context, pointer_down_at, pointer_move_at, pointer_up_at,
    };
    use crate::{
        set_current_theme, set_current_viewport, PaintCommands, PaintContext, Theme, ViewportInfo,
    };
    use sparsha_input::{
        with_shortcut_profile, FocusManager, InputEvent, KeyboardEvent, Modifiers, ShortcutProfile,
    };
    use sparsha_layout::LayoutTree;
    use sparsha_render::DrawList;
    use sparsha_text::TextSystem;
    use std::sync::{Arc, Mutex};

    fn prepare_input_with_cache(input: &TextInput) {
        let mut text = TextSystem::new_headless();
        let mut ctx = crate::LayoutContext {
            text: &mut text,
            max_width: None,
            max_height: None,
        };
        let _ = input.measure(&mut ctx);
    }

    fn reset_viewport() {
        set_current_viewport(ViewportInfo::default());
    }

    fn primary_modifiers() -> Modifiers {
        ShortcutProfile::ControlPrimary.primary_modifiers()
    }

    #[test]
    fn pointer_click_places_cursor_at_start_middle_end() {
        reset_viewport();
        let mut input = TextInput::builder().value("hello").build();
        input.set_id(Default::default());
        prepare_input_with_cache(&input);

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        let mut event_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);

        input.event(&mut event_ctx, &pointer_down_at(2.0, 18.0));
        assert_eq!(input.editor.cursor(), 0);

        let mid_prefix_width = input
            .line_metrics
            .borrow()
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == 3).then_some(*width))
            .unwrap_or_default();
        let mut move_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), true);
        input.event(
            &mut move_ctx,
            &pointer_move_at(mid_prefix_width + input.style.padding_h, 18.0),
        );
        assert_eq!(input.editor.cursor(), 3);

        let mut end_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
        input.event(&mut end_ctx, &pointer_down_at(238.0, 18.0));
        assert_eq!(input.editor.cursor(), input.get_value().len());
    }

    #[test]
    fn drag_selection_uses_pointer_capture() {
        reset_viewport();
        let mut input = TextInput::builder().value("hello world").build();
        input.set_id(Default::default());
        prepare_input_with_cache(&input);

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        let mut down_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
        input.event(&mut down_ctx, &pointer_down_at(2.0, 18.0));
        assert!(down_ctx.commands.capture_pointer);

        let width = input
            .line_metrics
            .borrow()
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == 5).then_some(*width))
            .unwrap_or_default();
        let mut move_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), true);
        input.event(
            &mut move_ctx,
            &pointer_move_at(width + input.style.padding_h, 18.0),
        );
        assert_eq!(input.editor.selection_range(), Some((0, 5)));

        let mut up_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), true);
        input.event(
            &mut up_ctx,
            &pointer_up_at(width + input.style.padding_h, 18.0),
        );
        assert!(up_ctx.commands.release_pointer);
    }

    #[test]
    fn copy_cut_paste_and_undo_roundtrip() {
        with_shortcut_profile(ShortcutProfile::ControlPrimary, || {
            reset_viewport();
            let changes = Arc::new(Mutex::new(Vec::new()));
            let changes_for_cb = Arc::clone(&changes);
            let mut input = TextInput::builder()
                .value("hello")
                .on_change(move |value| changes_for_cb.lock().unwrap().push(value.to_owned()))
                .build();
            input.set_id(Default::default());

            let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
            let layout_tree = LayoutTree::new();
            let mut focus = FocusManager::new();
            focus.set_focus(input.id());

            input.editor.select_all();
            let mut copy_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
            input.event(
                &mut copy_ctx,
                &InputEvent::KeyDown {
                    event: KeyboardEvent {
                        key: Key::Character("c".into()),
                        modifiers: primary_modifiers(),
                        ..Default::default()
                    },
                },
            );
            assert_eq!(copy_ctx.commands.clipboard_write.as_deref(), Some("hello"));

            let mut cut_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
            input.event(
                &mut cut_ctx,
                &InputEvent::KeyDown {
                    event: KeyboardEvent {
                        key: Key::Character("x".into()),
                        modifiers: primary_modifiers(),
                        ..Default::default()
                    },
                },
            );
            assert_eq!(input.get_value(), "");
            assert_eq!(cut_ctx.commands.clipboard_write.as_deref(), Some("hello"));

            let mut paste_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
            input.event(
                &mut paste_ctx,
                &InputEvent::Paste {
                    text: "world".to_owned(),
                },
            );
            assert_eq!(input.get_value(), "world");

            let mut undo_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
            input.event(
                &mut undo_ctx,
                &InputEvent::KeyDown {
                    event: KeyboardEvent {
                        key: Key::Character("z".into()),
                        modifiers: primary_modifiers(),
                        ..Default::default()
                    },
                },
            );
            assert_eq!(input.get_value(), "");

            let mut redo_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
            input.event(
                &mut redo_ctx,
                &InputEvent::KeyDown {
                    event: KeyboardEvent {
                        key: Key::Character("z".into()),
                        modifiers: primary_modifiers() | Modifiers::SHIFT,
                        ..Default::default()
                    },
                },
            );
            assert_eq!(input.get_value(), "world");
            assert!(!changes.lock().unwrap().is_empty());
        });
    }

    #[test]
    fn text_editor_state_matches_cursor_and_selection() {
        reset_viewport();
        let mut input = TextInput::builder().value("hello").build();
        input.editor.select_all();
        let state = input.text_editor_state().expect("text editor state");
        assert_eq!(state.text, "hello");
        assert_eq!(state.selection_range(), (0, 5));
        assert!(!state.multiline);
    }

    #[test]
    fn pointer_hit_testing_stays_correct_after_scaled_paint() {
        reset_viewport();
        let mut input = TextInput::builder().value("hello world").build();
        input.set_id(Default::default());

        let layout_tree = LayoutTree::new();
        let focus = FocusManager::new();
        let mut draw_list = DrawList::new();
        let mut text = TextSystem::new_headless();
        let mut commands = PaintCommands::default();
        let mut paint_ctx = PaintContext {
            draw_list: &mut draw_list,
            layout: layout_bounds(0.0, 0.0, 480.0, 72.0),
            layout_tree: &layout_tree,
            focus: &focus,
            widget_id: input.id(),
            scale_factor: 2.0,
            text_system: &mut text,
            elapsed_time: 0.0,
            commands: &mut commands,
        };
        input.paint(&mut paint_ctx);

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let mut event_focus = FocusManager::new();
        let mut event_ctx =
            mock_event_context(layout, &layout_tree, &mut event_focus, input.id(), false);
        input.event(&mut event_ctx, &pointer_down_at(238.0, 18.0));

        assert_eq!(input.editor.cursor(), input.get_value().len());
    }

    #[test]
    fn space_text_input_advances_cursor_and_updates_value() {
        reset_viewport();
        let mut input = TextInput::builder().value("hey").build();
        input.set_id(Default::default());
        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(input.id());

        let mut key_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
        input.event(
            &mut key_ctx,
            &InputEvent::KeyDown {
                event: KeyboardEvent::key_down(
                    Key::Character(" ".to_owned()),
                    sparsha_input::ui_events::keyboard::Code::Space,
                ),
            },
        );

        let mut text_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
        input.event(
            &mut text_ctx,
            &InputEvent::TextInput {
                text: " ".to_owned(),
            },
        );

        assert_eq!(input.get_value(), "hey ");
        assert_eq!(input.editor.cursor(), input.get_value().len());
    }

    #[test]
    fn line_metrics_track_trailing_space_width() {
        reset_viewport();
        let input = TextInput::builder().value("a ").build();
        prepare_input_with_cache(&input);

        let prefix = input.line_metrics.borrow();
        let width_after_a = prefix
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == 1).then_some(*width))
            .expect("width after first glyph");
        let width_after_space = prefix
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == 2).then_some(*width))
            .expect("width after trailing space");

        assert!(width_after_space > width_after_a);
    }

    #[test]
    fn single_line_measurement_stays_single_line_under_width_constraint() {
        reset_viewport();
        let input = TextInput::builder()
            .value("this should remain on one line")
            .build();

        let mut text = TextSystem::new_headless();
        let mut constrained = crate::LayoutContext {
            text: &mut text,
            max_width: Some(40.0),
            max_height: None,
        };
        let constrained_size = input.measure(&mut constrained).expect("constrained size");

        let mut text = TextSystem::new_headless();
        let mut unconstrained = crate::LayoutContext {
            text: &mut text,
            max_width: None,
            max_height: None,
        };
        let unconstrained_size = input
            .measure(&mut unconstrained)
            .expect("unconstrained size");

        assert_eq!(constrained_size, unconstrained_size);
    }

    #[test]
    fn themed_defaults_scale_down_for_mobile_viewport() {
        let mut theme = Theme::default();
        theme.typography.body_size = 16.0;
        theme.controls.control_height = 38.0;
        theme.controls.control_padding_x = 12.0;
        set_current_theme(theme);
        set_current_viewport(ViewportInfo::new(390.0, 844.0));

        let input = TextInput::builder().build();
        let style = input.resolved_style();
        let epsilon = f32::EPSILON;
        assert!((style.font_size - 14.0).abs() <= epsilon);
        assert!((style.min_height - 34.0).abs() <= epsilon);
        assert!((style.padding_h - 10.0).abs() <= epsilon);
    }

    #[test]
    fn builder_sets_explicit_configuration() {
        let style = TextInputStyle {
            min_width: 240.0,
            ..TextInputStyle::default()
        };

        let input = TextInput::builder()
            .value("Builder")
            .placeholder("Type")
            .style(style.clone())
            .fill_width(true)
            .build();

        assert_eq!(input.get_value(), "Builder");
        assert_eq!(input.placeholder, "Type");
        assert_eq!(input.style.min_width, style.min_width);
        assert!(input.fill_width);
        assert!(!input.use_theme_defaults);
    }
}
