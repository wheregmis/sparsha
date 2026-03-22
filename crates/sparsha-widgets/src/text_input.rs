//! Text input widget.

use crate::text_editor::{EditorCore, TextEditorState};
use crate::{
    control_state::{focus_ring_border_width, focus_ring_bounds, focus_ring_color},
    current_theme, responsive_theme_controls, responsive_typography, AccessibilityAction,
    AccessibilityInfo, AccessibilityRole, EventContext, PaintContext, Widget,
};
use sparsha_core::Color;
use sparsha_input::{Action, ActionMapper, InputEvent, Key, NamedKey, StandardAction};
use sparsha_layout::WidgetId;
use sparsha_text::TextStyle;
use std::cell::RefCell;
use taffy::prelude::*;

/// Callback type for text change and submit handlers.
type TextInputCallback = Box<dyn FnMut(&str)>;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static TEXT_MEASURE_SPAN: RefCell<Option<web_sys::Element>> = const { RefCell::new(None) };
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
    prefix_widths: RefCell<Vec<(usize, f32)>>,
}

#[derive(Clone)]
struct TextInputBuildState {
    editor: EditorCore,
    declared_value: Option<String>,
}

impl TextInput {
    /// Create a new text input.
    pub fn new() -> Self {
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
            prefix_widths: RefCell::new(vec![(0, 0.0)]),
        }
    }

    /// Set the initial value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.editor.set_text(value.clone());
        self.declared_value = Some(value);
        self
    }

    /// Set the placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the change handler.
    pub fn on_change(mut self, handler: impl FnMut(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    /// Set the submit handler (called on Enter).
    pub fn on_submit(mut self, handler: impl FnMut(&str) + 'static) -> Self {
        self.on_submit = Some(Box::new(handler));
        self
    }

    /// Set the style.
    pub fn with_style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self.use_theme_defaults = false;
        self
    }

    /// Stretch to fill the parent's available width.
    pub fn fill_width(mut self) -> Self {
        self.fill_width = true;
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
        let cache = Self::compute_prefix_widths(self.editor.text(), |prefix| {
            self.measure_width(ctx, style, prefix) / ctx.scale_factor.max(1.0)
        });
        *self.prefix_widths.borrow_mut() = cache;
    }

    fn update_prefix_width_cache_with_layout_ctx(
        &self,
        ctx: &mut crate::LayoutContext,
        style: &TextStyle,
    ) {
        let cache = Self::compute_prefix_widths(self.editor.text(), |prefix| {
            ctx.measure_text(prefix, style).0
        });
        *self.prefix_widths.borrow_mut() = cache;
    }

    fn cursor_index_for_x(&self, x: f32) -> usize {
        if self.editor.text().is_empty() {
            return 0;
        }
        let prefix = self.prefix_widths.borrow();
        if prefix.is_empty() {
            return self.editor.text().len();
        }

        if x <= 0.0 {
            return 0;
        }
        if let Some((last_idx, last_x)) = prefix.last() {
            if x >= *last_x {
                return *last_idx;
            }
        }

        let mut best = (self.editor.text().len(), f32::MAX);
        for (idx, width) in prefix.iter() {
            let dist = (*width - x).abs();
            if dist < best.1 {
                best = (*idx, dist);
            }
        }
        best.0
    }

    fn prefix_width_for(&self, index: usize) -> Option<f32> {
        self.prefix_widths
            .borrow()
            .iter()
            .find_map(|(idx, width)| (*idx == index).then_some(*width))
    }

    fn cursor_offset_for(&self, ctx: &mut PaintContext, style: &TextStyle) -> f32 {
        if let Some(width) = self.prefix_width_for(self.editor.cursor()) {
            return width * ctx.scale_factor;
        }
        let text_before_cursor = &self.editor.text()[..self.editor.cursor()];
        self.measure_width(ctx, style, text_before_cursor)
    }

    fn measure_width(&self, ctx: &mut PaintContext, style: &TextStyle, text: &str) -> f32 {
        #[cfg(target_arch = "wasm32")]
        if let Some(width) = measure_text_width_dom(text, style, ctx.scale_factor) {
            return width;
        }
        ctx.measure_text(text, style).0
    }

    fn themed_default_style() -> TextInputStyle {
        let theme = current_theme();
        let controls = responsive_theme_controls(&theme);
        let typography = responsive_typography(&theme);
        TextInputStyle {
            background: theme.colors.input_background,
            background_focused: theme.colors.surface,
            text_color: theme.colors.text_primary,
            placeholder_color: theme.colors.input_placeholder,
            border_color: theme.colors.border,
            border_color_focused: theme.colors.primary,
            border_width: 1.0,
            corner_radius: theme.radii.md,
            padding_h: controls.control_padding_x,
            padding_v: controls.control_padding_y,
            font_size: typography.body_size,
            min_width: 180.0,
            min_height: controls.control_height,
        }
    }

    fn resolved_style(&self) -> TextInputStyle {
        if self.use_theme_defaults {
            Self::themed_default_style()
        } else {
            self.style.clone()
        }
    }

    fn handle_action(&mut self, ctx: &mut EventContext, action: StandardAction) -> bool {
        let changed = match action {
            StandardAction::SelectAll => {
                self.editor.select_all();
                ctx.request_paint();
                false
            }
            StandardAction::Copy => {
                if let Some(text) = self.editor.copy_selection() {
                    ctx.write_clipboard(text);
                }
                ctx.request_paint();
                false
            }
            StandardAction::Cut => {
                if let Some(text) = self.editor.cut_selection() {
                    ctx.write_clipboard(text);
                    self.fire_change();
                    ctx.request_layout();
                    true
                } else {
                    false
                }
            }
            StandardAction::Undo => {
                let changed = self.editor.undo();
                if changed {
                    self.fire_change();
                    ctx.request_layout();
                }
                changed
            }
            StandardAction::Redo => {
                let changed = self.editor.redo();
                if changed {
                    self.fire_change();
                    ctx.request_layout();
                }
                changed
            }
            StandardAction::Backspace => {
                let changed = self.editor.backspace();
                if changed {
                    self.fire_change();
                    ctx.request_layout();
                }
                changed
            }
            StandardAction::Delete => {
                let changed = self.editor.delete_forward();
                if changed {
                    self.fire_change();
                    ctx.request_layout();
                }
                changed
            }
            StandardAction::MoveLeft => {
                self.editor.move_left(false);
                ctx.request_paint();
                false
            }
            StandardAction::MoveRight => {
                self.editor.move_right(false);
                ctx.request_paint();
                false
            }
            StandardAction::SelectLeft => {
                self.editor.move_left(true);
                ctx.request_paint();
                false
            }
            StandardAction::SelectRight => {
                self.editor.move_right(true);
                ctx.request_paint();
                false
            }
            StandardAction::MoveWordLeft => {
                self.editor.move_word_left(false);
                ctx.request_paint();
                false
            }
            StandardAction::MoveWordRight => {
                self.editor.move_word_right(false);
                ctx.request_paint();
                false
            }
            StandardAction::SelectWordLeft => {
                self.editor.move_word_left(true);
                ctx.request_paint();
                false
            }
            StandardAction::SelectWordRight => {
                self.editor.move_word_right(true);
                ctx.request_paint();
                false
            }
            StandardAction::MoveToStart => {
                self.editor.move_to_start(false);
                ctx.request_paint();
                false
            }
            StandardAction::MoveToEnd => {
                self.editor.move_to_end(false);
                ctx.request_paint();
                false
            }
            StandardAction::SelectToStart => {
                self.editor.move_to_start(true);
                ctx.request_paint();
                false
            }
            StandardAction::SelectToEnd => {
                self.editor.move_to_end(true);
                ctx.request_paint();
                false
            }
            _ => false,
        };

        ctx.stop_propagation();
        changed
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

    fn rebuild(&mut self, ctx: &mut crate::BuildContext) {
        if let Some(state) = ctx
            .take_boxed_state()
            .and_then(|state| state.downcast::<TextInputBuildState>().ok())
            .map(|state| *state)
        {
            if state.declared_value == self.declared_value {
                self.editor = state.editor;
            }
        }

        ctx.store_boxed_state(Box::new(TextInputBuildState {
            editor: self.editor.clone(),
            declared_value: self.declared_value.clone(),
        }));
    }

    fn persist_build_state(&self, ctx: &mut crate::BuildContext) {
        ctx.store_boxed_state(Box::new(TextInputBuildState {
            editor: self.editor.clone(),
            declared_value: self.declared_value.clone(),
        }));
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
        let text_x = bounds.x + padding_h;
        let text_width = bounds.width - padding_h * 2.0;

        let text_style = TextStyle::default()
            .with_family(current_theme().typography.font_family.clone())
            .with_size(style.font_size)
            .with_color(style.text_color);

        let placeholder_style = TextStyle::default()
            .with_family(current_theme().typography.font_family)
            .with_size(style.font_size)
            .with_color(style.placeholder_color);

        self.update_prefix_width_cache_with_paint_ctx(ctx, &text_style);

        let (_, text_height) = ctx.measure_text("Ay", &text_style);
        let text_y = bounds.y + (bounds.height - text_height) / 2.0;

        if self.editor.text().is_empty() {
            if !self.placeholder.is_empty() {
                ctx.draw_text(&self.placeholder, &placeholder_style, text_x, text_y);
            }
        } else {
            if let Some((start, end)) = self.editor.selection_range() {
                let text_before_sel = &self.editor.text()[..start];
                let sel_x_start = self.measure_width(ctx, &text_style, text_before_sel);
                let selected_text = &self.editor.text()[start..end];
                let sel_width = self.measure_width(ctx, &text_style, selected_text);
                if sel_width > 0.0 {
                    let sel_rect = sparsha_core::Rect::new(
                        text_x + sel_x_start,
                        text_y,
                        sel_width.min(text_width - sel_x_start),
                        text_height,
                    );
                    ctx.fill_rect(sel_rect, Color::from_hex(0x3B82F6).with_alpha(0.3));
                }
            }

            ctx.draw_text(self.editor.text(), &text_style, text_x, text_y);
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
        let text_style = TextStyle::default()
            .with_family(current_theme().typography.font_family)
            .with_size(style.font_size);
        self.update_prefix_width_cache_with_layout_ctx(ctx, &text_style);
        let sample = if self.editor.text().is_empty() {
            if self.placeholder.is_empty() {
                "M"
            } else {
                &self.placeholder
            }
        } else {
            self.editor.text()
        };
        let (text_width, text_height) = ctx.measure_text(sample, &text_style);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        layout_bounds, mock_event_context, pointer_down_at, pointer_move_at, pointer_up_at,
    };
    use crate::{
        set_current_theme, set_current_viewport, PaintCommands, PaintContext, Theme, ViewportInfo,
    };
    use sparsha_input::{FocusManager, InputEvent, KeyboardEvent, Modifiers};
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
    fn pointer_click_places_cursor_at_start_middle_end() {
        reset_viewport();
        let mut input = TextInput::new().value("hello");
        input.set_id(Default::default());
        prepare_input_with_cache(&input);

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        let mut event_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);

        input.event(&mut event_ctx, &pointer_down_at(2.0, 18.0));
        assert_eq!(input.editor.cursor(), 0);

        let mid_prefix_width = input
            .prefix_widths
            .borrow()
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
        let mut input = TextInput::new().value("hello world");
        input.set_id(Default::default());
        prepare_input_with_cache(&input);

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        let mut down_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
        input.event(&mut down_ctx, &pointer_down_at(2.0, 18.0));
        assert!(down_ctx.commands.capture_pointer);

        let width = input
            .prefix_widths
            .borrow()
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
        reset_viewport();
        let changes = Arc::new(Mutex::new(Vec::new()));
        let changes_for_cb = Arc::clone(&changes);
        let mut input = TextInput::new()
            .value("hello")
            .on_change(move |value| changes_for_cb.lock().unwrap().push(value.to_owned()));
        input.set_id(Default::default());

        let layout = layout_bounds(0.0, 0.0, 240.0, 36.0);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(input.id());

        input.editor.select_all();
        let mut copy_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
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

        let mut cut_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
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

        let mut paste_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
        input.event(
            &mut paste_ctx,
            &InputEvent::Paste {
                text: "world".to_owned(),
            },
        );
        assert_eq!(input.get_value(), "world");

        let mut undo_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
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

        let mut redo_ctx = mock_event_context(layout, &layout_tree, &mut focus, input.id(), false);
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
    }

    #[test]
    fn text_editor_state_matches_cursor_and_selection() {
        reset_viewport();
        let mut input = TextInput::new().value("hello");
        input.editor.select_all();
        let state = input.text_editor_state().expect("text editor state");
        assert_eq!(state.text, "hello");
        assert_eq!(state.selection_range(), (0, 5));
        assert!(!state.multiline);
    }

    #[test]
    fn pointer_hit_testing_stays_correct_after_scaled_paint() {
        reset_viewport();
        let mut input = TextInput::new().value("hello world");
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
        let mut input = TextInput::new().value("hey");
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
    fn prefix_cache_tracks_trailing_space_width() {
        reset_viewport();
        let input = TextInput::new().value("a ");
        prepare_input_with_cache(&input);

        let prefix = input.prefix_widths.borrow();
        let width_after_a = prefix
            .iter()
            .find_map(|(idx, width)| (*idx == 1).then_some(*width))
            .expect("width after first glyph");
        let width_after_space = prefix
            .iter()
            .find_map(|(idx, width)| (*idx == 2).then_some(*width))
            .expect("width after trailing space");

        assert!(width_after_space > width_after_a);
    }

    #[test]
    fn themed_defaults_scale_down_for_mobile_viewport() {
        let mut theme = Theme::default();
        theme.typography.body_size = 16.0;
        theme.controls.control_height = 38.0;
        theme.controls.control_padding_x = 12.0;
        set_current_theme(theme);
        set_current_viewport(ViewportInfo::new(390.0, 844.0));

        let input = TextInput::new();
        let style = input.resolved_style();
        let epsilon = f32::EPSILON;
        assert!((style.font_size - 14.0).abs() <= epsilon);
        assert!((style.min_height - 34.0).abs() <= epsilon);
        assert!((style.padding_h - 10.0).abs() <= epsilon);
    }
}
