//! Multiline text area widget.

use crate::text_editor::{EditorCore, TextEditorState};
use crate::text_editor_widget::{
    apply_standard_action, compute_prefix_widths, editor_placeholder_style, editor_text_style,
    editor_widget_style, paint_editor_frame, persist_editor_build_state, resolve_editor_style,
    restore_editor_build_state, set_editor_value,
};
use crate::text_input::TextInputStyle;
use crate::{
    AccessibilityAction, AccessibilityInfo, AccessibilityRole, EventContext, PaintContext, Widget,
};
use bon::bon;
use sparsha_core::Color;
use sparsha_input::{Action, ActionMapper, InputEvent, Key, NamedKey, StandardAction};
use sparsha_layout::WidgetId;
use sparsha_text::{TextBreakMode, TextLayoutInfo, TextLayoutOptions, TextStyle, TextWrap};
use std::cell::RefCell;
use taffy::prelude::*;

type TextAreaCallback = Box<dyn FnMut(&str)>;
pub type TextAreaStyle = TextInputStyle;

#[derive(Clone, Debug)]
struct LineMetrics {
    start: usize,
    end: usize,
    x_offset: f32,
    y_offset: f32,
    height: f32,
    widths: Vec<(usize, f32)>,
}

/// A multiline text editor widget.
pub struct TextArea {
    id: WidgetId,
    editor: EditorCore,
    declared_value: Option<String>,
    placeholder: String,
    style: TextAreaStyle,
    on_change: Option<TextAreaCallback>,
    fill_width: bool,
    use_theme_defaults: bool,
    line_metrics: RefCell<Vec<LineMetrics>>,
}

impl TextArea {
    fn empty() -> Self {
        Self {
            id: WidgetId::default(),
            editor: EditorCore::new(String::new()),
            declared_value: None,
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
                x_offset: 0.0,
                y_offset: 0.0,
                height: 0.0,
                widths: vec![(0, 0.0)],
            }]),
        }
    }

    fn style_override(mut self, style: TextAreaStyle) -> Self {
        self.style = style;
        self.use_theme_defaults = false;
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
        resolve_editor_style(&self.style, self.use_theme_defaults, true)
    }

    fn update_line_metrics(
        &self,
        layout_info: &TextLayoutInfo,
        style: &TextStyle,
        mut measure_prefix: impl FnMut(&str) -> f32,
    ) {
        if self.editor.text().is_empty() {
            *self.line_metrics.borrow_mut() = vec![LineMetrics {
                start: 0,
                end: 0,
                x_offset: 0.0,
                y_offset: 0.0,
                height: style.font_size * style.line_height,
                widths: vec![(0, 0.0)],
            }];
            return;
        }

        let text = self.editor.text();
        let metrics = layout_info
            .lines
            .iter()
            .map(|line| {
                let slice = &text[line.text_range.clone()];
                LineMetrics {
                    start: line.text_range.start,
                    end: line.text_range.end,
                    x_offset: line.offset,
                    y_offset: line.min_coord,
                    height: line.line_height,
                    widths: compute_prefix_widths(slice, line.text_range.start, |prefix| {
                        measure_prefix(prefix)
                    }),
                }
            })
            .collect();
        *self.line_metrics.borrow_mut() = metrics;
    }

    fn update_line_metrics_with_layout_ctx(
        &self,
        ctx: &mut crate::LayoutContext,
        style: &TextStyle,
        max_width: Option<f32>,
    ) {
        let layout = ctx.text.layout_info(
            self.editor.text(),
            style,
            TextLayoutOptions::new()
                .with_max_width(max_width)
                .with_wrap(TextWrap::Word),
        );
        self.update_line_metrics(&layout, style, |prefix| {
            ctx.text.measure(prefix, style, None).0
        });
    }

    fn update_line_metrics_with_paint_ctx(
        &self,
        ctx: &mut PaintContext,
        style: &TextStyle,
        max_width: Option<f32>,
    ) {
        let layout = ctx.text_system.layout_info(
            self.editor.text(),
            style,
            TextLayoutOptions::new()
                .with_max_width(max_width)
                .with_wrap(TextWrap::Word),
        );
        self.update_line_metrics(&layout, style, |prefix| {
            ctx.text_system.measure(prefix, style, None).0
        });
    }

    fn cursor_index_for_position(&self, x: f32, y: f32) -> usize {
        let metrics = self.line_metrics.borrow();
        if metrics.is_empty() {
            return 0;
        }
        let Some(line_metrics) = metrics
            .iter()
            .find(|line| y < line.y_offset + line.height)
            .or_else(|| metrics.last())
        else {
            return 0;
        };

        if x <= line_metrics.x_offset {
            return line_metrics.start;
        }
        let local_x = (x - line_metrics.x_offset).max(0.0);
        if let Some((last_idx, last_x)) = line_metrics.widths.last() {
            if local_x >= *last_x {
                return *last_idx;
            }
        }

        let mut best = (line_metrics.end, f32::MAX);
        for (idx, width) in &line_metrics.widths {
            let distance = (*width - local_x).abs();
            if distance < best.1 {
                best = (*idx, distance);
            }
        }
        best.0
    }

    fn handle_action(&mut self, ctx: &mut EventContext, action: StandardAction) {
        if apply_standard_action(&mut self.editor, ctx, action, true) {
            self.fire_change();
        }
    }
}

#[bon]
impl TextArea {
    #[builder(
        start_fn(name = builder, vis = "pub"),
        finish_fn(name = build, vis = "pub"),
        builder_type(name = TextAreaBuilder, vis = "pub"),
        state_mod(vis = "pub")
    )]
    fn builder_init(
        #[builder(with = |value: impl Into<String>| value.into())] value: Option<String>,
        #[builder(into, default = String::new())] placeholder: String,
        style: Option<TextAreaStyle>,
        #[builder(default)] fill_width: bool,
        #[builder(with = |handler: impl FnMut(&str) + 'static| Box::new(handler) as TextAreaCallback)]
        on_change: Option<TextAreaCallback>,
    ) -> Self {
        let mut area = Self::empty();
        if let Some(value) = value {
            area.editor.set_text(value.clone());
            area.declared_value = Some(value);
        }
        area.placeholder = placeholder;
        if let Some(style) = style {
            area = area.style_override(style);
        }
        area.fill_width = fill_width;
        if let Some(on_change) = on_change {
            area.on_change = Some(on_change);
        }
        area
    }
}

impl Widget for TextArea {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn rebuild(&mut self, ctx: &mut crate::BuildContext) {
        restore_editor_build_state(&mut self.editor, &self.declared_value, true, ctx);
        persist_editor_build_state(&self.editor, &self.declared_value, true, ctx);
    }

    fn persist_build_state(&self, ctx: &mut crate::BuildContext) {
        persist_editor_build_state(&self.editor, &self.declared_value, true, ctx);
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
        let padding_v = style.padding_v * scale;
        let text_x = bounds.x + padding_h;
        let text_y = bounds.y + padding_v;
        let content_width = ((bounds.width / scale) - style.padding_h * 2.0).max(0.0);
        let content_bounds = sparsha_core::Rect::new(
            text_x,
            text_y,
            (bounds.width - padding_h * 2.0).max(0.0),
            (bounds.height - padding_v * 2.0).max(0.0),
        );
        let text_style = editor_text_style(&style);
        let placeholder_style = editor_placeholder_style(&style);

        self.update_line_metrics_with_paint_ctx(ctx, &text_style, Some(content_width));

        if self.editor.text().is_empty() {
            if !self.placeholder.is_empty() {
                ctx.draw_text_block(
                    &self.placeholder,
                    &placeholder_style,
                    content_bounds,
                    TextWrap::Word,
                    TextBreakMode::Normal,
                    sparsha_text::TextLayoutAlignment::Start,
                    None,
                );
            }
        } else {
            if let Some((selection_start, selection_end)) = self.editor.selection_range() {
                for line_metrics in self.line_metrics.borrow().iter() {
                    let highlight_start = selection_start.max(line_metrics.start);
                    let highlight_end = selection_end.min(line_metrics.end);
                    if highlight_start >= highlight_end {
                        continue;
                    }

                    let start_x = line_metrics
                        .widths
                        .iter()
                        .find_map(|(idx, width)| (*idx == highlight_start).then_some(*width))
                        .unwrap_or_default();
                    let end_x = line_metrics
                        .widths
                        .iter()
                        .find_map(|(idx, width)| (*idx == highlight_end).then_some(*width))
                        .unwrap_or(start_x);

                    ctx.fill_rect(
                        sparsha_core::Rect::new(
                            text_x + (line_metrics.x_offset + start_x) * scale,
                            text_y + line_metrics.y_offset * scale,
                            (end_x - start_x).max(0.0) * scale,
                            line_metrics.height * scale,
                        ),
                        Color::from_hex(0x3B82F6).with_alpha(0.3),
                    );
                }
            }

            ctx.draw_text_block(
                self.editor.text(),
                &text_style,
                content_bounds,
                TextWrap::Word,
                TextBreakMode::Normal,
                sparsha_text::TextLayoutAlignment::Start,
                None,
            );
        }

        if focused {
            ctx.request_next_frame();
            let cursor_visible = (ctx.elapsed_time * 2.0).fract() < 0.5;
            if cursor_visible {
                let line_metrics = self
                    .line_metrics
                    .borrow()
                    .iter()
                    .find(|line| {
                        self.editor.cursor() >= line.start && self.editor.cursor() <= line.end
                    })
                    .cloned()
                    .or_else(|| self.line_metrics.borrow().last().cloned())
                    .unwrap_or(LineMetrics {
                        start: 0,
                        end: 0,
                        x_offset: 0.0,
                        y_offset: 0.0,
                        height: text_style.font_size * text_style.line_height,
                        widths: vec![(0, 0.0)],
                    });
                let cursor_x = line_metrics
                    .widths
                    .iter()
                    .find_map(|(idx, width)| (*idx == self.editor.cursor()).then_some(*width))
                    .unwrap_or_default();
                ctx.fill_rect(
                    sparsha_core::Rect::new(
                        text_x + (line_metrics.x_offset + cursor_x) * scale,
                        text_y + line_metrics.y_offset * scale,
                        2.0 * scale,
                        line_metrics.height * scale,
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
                let x = (local.x - style.padding_h).max(0.0);
                let y = (local.y - style.padding_v).max(0.0);
                let index = self.cursor_index_for_position(x, y);
                self.editor.set_cursor(index, false);
                ctx.capture_pointer();
            }
            InputEvent::PointerMove { pos } if ctx.has_capture => {
                let style = self.resolved_style();
                let local = ctx.to_local(*pos);
                let x = (local.x - style.padding_h).max(0.0);
                let y = (local.y - style.padding_v).max(0.0);
                let index = self.cursor_index_for_position(x, y);
                self.editor.set_cursor(index, true);
                ctx.request_paint();
            }
            InputEvent::PointerUp { pos, .. } if ctx.has_capture => {
                let style = self.resolved_style();
                let local = ctx.to_local(*pos);
                let x = (local.x - style.padding_h).max(0.0);
                let y = (local.y - style.padding_v).max(0.0);
                let index = self.cursor_index_for_position(x, y);
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
        let text_style = editor_text_style(&style);
        let horizontal_chrome = style.padding_h * 2.0 + style.border_width * 2.0;
        let vertical_chrome = style.padding_v * 2.0 + style.border_width * 2.0;
        let content_width = ctx
            .max_width
            .map(|width| (width - horizontal_chrome).max(0.0))
            .filter(|width| *width > 0.0);
        self.update_line_metrics_with_layout_ctx(ctx, &text_style, content_width);

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
                .with_max_width(content_width)
                .with_wrap(TextWrap::Word),
        );

        let width = (layout.width + horizontal_chrome).max(style.min_width);
        let height = (layout.height + vertical_chrome).max(style.min_height);
        Some((width, height))
    }

    fn requires_post_layout_measurement(&self) -> bool {
        true
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
    use crate::test_helpers::{layout_bounds, mock_event_context};
    use crate::{
        set_current_theme, set_current_viewport, PaintCommands, PaintContext, Theme, ViewportInfo,
    };
    use sparsha_input::{
        with_shortcut_profile, FocusManager, InputEvent, KeyboardEvent, Modifiers, ShortcutProfile,
    };
    use sparsha_layout::LayoutTree;
    use sparsha_render::DrawList;
    use sparsha_text::TextSystem;

    fn prepare_area_with_cache(area: &TextArea) {
        let mut text = TextSystem::new_headless();
        let mut ctx = crate::LayoutContext {
            text: &mut text,
            max_width: None,
            max_height: None,
        };
        let _ = area.measure(&mut ctx);
    }

    fn prepare_area_with_width(area: &TextArea, max_width: f32) -> (f32, f32) {
        let mut text = TextSystem::new_headless();
        let mut ctx = crate::LayoutContext {
            text: &mut text,
            max_width: Some(max_width),
            max_height: None,
        };
        area.measure(&mut ctx).expect("text area measurement")
    }

    fn reset_viewport() {
        set_current_viewport(ViewportInfo::default());
    }

    fn primary_modifiers() -> Modifiers {
        ShortcutProfile::ControlPrimary.primary_modifiers()
    }

    #[test]
    fn enter_inserts_newline_and_vertical_navigation_moves_cursor() {
        reset_viewport();
        let mut area = TextArea::builder().value("hello").build();
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
        with_shortcut_profile(ShortcutProfile::ControlPrimary, || {
            reset_viewport();
            let mut area = TextArea::builder().build();
            area.set_id(Default::default());
            let layout = layout_bounds(0.0, 0.0, 280.0, 120.0);
            let layout_tree = LayoutTree::new();
            let mut focus = FocusManager::new();
            focus.set_focus(area.id());

            let mut paste_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
            area.event(
                &mut paste_ctx,
                &InputEvent::Paste {
                    text: "alpha\nbeta".to_owned(),
                },
            );
            assert_eq!(area.get_value(), "alpha\nbeta");

            let mut undo_ctx =
                mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
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
        });
    }

    #[test]
    fn pointer_hit_testing_stays_correct_after_scaled_paint() {
        reset_viewport();
        let mut area = TextArea::builder().value("line one\nline two").build();
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
                button: sparsha_input::PointerButton::Primary,
            },
        );

        assert_eq!(area.editor.cursor(), area.get_value().len());
    }

    #[test]
    fn trailing_space_text_input_advances_cursor() {
        reset_viewport();
        let mut area = TextArea::builder().value("hello").build();
        area.set_id(Default::default());
        let layout = layout_bounds(0.0, 0.0, 280.0, 120.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(area.id());

        let mut key_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
        area.event(
            &mut key_ctx,
            &InputEvent::KeyDown {
                event: KeyboardEvent::key_down(
                    Key::Character(" ".to_owned()),
                    sparsha_input::ui_events::keyboard::Code::Space,
                ),
            },
        );

        let mut text_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);
        area.event(
            &mut text_ctx,
            &InputEvent::TextInput {
                text: " ".to_owned(),
            },
        );

        assert_eq!(area.get_value(), "hello ");
        assert_eq!(area.editor.cursor(), area.get_value().len());
    }

    #[test]
    fn line_metrics_track_trailing_space_width() {
        reset_viewport();
        let area = TextArea::builder().value("a \nline").build();
        prepare_area_with_cache(&area);

        let metrics = area.line_metrics.borrow();
        let first_line = metrics.first().expect("first line metrics");
        let width_after_a = first_line
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == 1).then_some(*width))
            .expect("width after first glyph");
        let width_after_space = first_line
            .widths
            .iter()
            .find_map(|(idx, width)| (*idx == 2).then_some(*width))
            .expect("width after trailing space");

        assert!(width_after_space > width_after_a);
    }

    #[test]
    fn wrapped_multiline_measurement_grows_height_when_width_is_constrained() {
        reset_viewport();
        let area = TextArea::builder()
            .value("alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu")
            .fill_width(true)
            .build();

        let unconstrained = prepare_area_with_width(&area, 480.0);
        let constrained = prepare_area_with_width(&area, 120.0);

        assert!(constrained.1 > unconstrained.1);
        assert!(area.line_metrics.borrow().len() > 1);
    }

    #[test]
    fn wrapped_multiline_hit_testing_tracks_visual_lines() {
        reset_viewport();
        let mut area = TextArea::builder()
            .value("alpha beta gamma delta epsilon zeta")
            .fill_width(true)
            .build();
        area.set_id(Default::default());
        let _ = prepare_area_with_width(&area, 180.0);

        let second_line = area
            .line_metrics
            .borrow()
            .get(1)
            .cloned()
            .expect("second visual line");
        let layout = layout_bounds(0.0, 0.0, 180.0, 220.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        let mut event_ctx = mock_event_context(layout, &layout_tree, &mut focus, area.id(), false);

        area.event(
            &mut event_ctx,
            &InputEvent::PointerDown {
                pos: glam::vec2(
                    second_line.x_offset + 2.0,
                    second_line.y_offset + second_line.height * 0.5,
                ),
                button: sparsha_input::PointerButton::Primary,
            },
        );

        assert!(area.editor.cursor() >= second_line.start);
        assert!(area.editor.cursor() <= second_line.end);
    }

    #[test]
    fn themed_defaults_scale_down_for_mobile_viewport() {
        let mut theme = Theme::default();
        theme.typography.body_size = 16.0;
        theme.controls.control_height = 38.0;
        theme.controls.control_padding_y = 8.0;
        set_current_theme(theme);
        set_current_viewport(ViewportInfo::new(390.0, 844.0));

        let area = TextArea::builder().build();
        let style = area.resolved_style();
        assert!(style.font_size < 16.0);
        assert!(style.min_height < 96.0);
        assert!(style.padding_v < 8.0);
    }

    #[test]
    fn builder_sets_explicit_configuration() {
        let style = TextAreaStyle {
            min_height: 140.0,
            ..TextAreaStyle::default()
        };

        let area = TextArea::builder()
            .value("Builder")
            .placeholder("More text")
            .style(style.clone())
            .fill_width(true)
            .build();

        assert_eq!(area.get_value(), "Builder");
        assert_eq!(area.placeholder, "More text");
        assert_eq!(area.style.min_height, style.min_height);
        assert!(area.fill_width);
        assert!(!area.use_theme_defaults);
    }
}
