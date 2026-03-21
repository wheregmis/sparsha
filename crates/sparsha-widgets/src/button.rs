//! Button widget.

use crate::{
    control_state::{focus_ring_border_width, focus_ring_bounds, focus_ring_color, ControlState},
    current_theme, AccessibilityAction, AccessibilityInfo, AccessibilityRole, EventContext,
    PaintContext, Widget,
};
use sparsha_core::Color;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
use sparsha_text::TextStyle;
use taffy::prelude::*;

/// Visual state of the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

/// Style configuration for a button.
#[derive(Clone, Debug)]
pub struct ButtonStyle {
    pub background: Color,
    pub background_hovered: Color,
    pub background_pressed: Color,
    pub background_disabled: Color,
    pub text_color: Color,
    pub text_color_disabled: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub corner_radius: f32,
    pub padding_h: f32,
    pub padding_v: f32,
    pub font_size: f32,
    /// Minimum width (0 = auto based on content)
    pub min_width: f32,
    /// Minimum height (0 = auto based on content)  
    pub min_height: f32,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            background: Color::from_hex(0x3B82F6),          // Blue
            background_hovered: Color::from_hex(0x2563EB),  // Darker blue
            background_pressed: Color::from_hex(0x1D4ED8),  // Even darker
            background_disabled: Color::from_hex(0x9CA3AF), // Gray
            text_color: Color::WHITE,
            text_color_disabled: Color::from_hex(0x6B7280),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            corner_radius: 6.0,
            padding_h: 12.0,
            padding_v: 8.0,
            font_size: 14.0,
            min_width: 0.0, // Will be set based on label
            min_height: 38.0,
        }
    }
}

/// A clickable button widget.
pub struct Button {
    id: WidgetId,
    label: String,
    style_override: Option<ButtonStyle>,
    background_override: Option<Color>,
    text_color_override: Option<Color>,
    corner_radius_override: Option<f32>,
    disabled: bool,
    interaction: ControlState,
    on_click: Option<Box<dyn FnMut()>>,
}

impl Button {
    /// Create a new button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: WidgetId::default(),
            label: label.into(),
            style_override: None,
            background_override: None,
            text_color_override: None,
            corner_radius_override: None,
            disabled: false,
            interaction: ControlState::default(),
            on_click: None,
        }
    }

    /// Set the click handler.
    pub fn on_click(mut self, handler: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Set the button style.
    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style_override = Some(style);
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background_override = Some(color);
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color_override = Some(color);
        self
    }

    /// Set corner radius.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius_override = Some(radius);
        self
    }

    /// Disable the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        if disabled {
            self.interaction.clear_interaction();
        }
        self
    }

    /// Current visual state (for tests and debugging).
    pub fn state(&self) -> ButtonState {
        if self.disabled {
            ButtonState::Disabled
        } else if self.interaction.pressed() {
            ButtonState::Pressed
        } else if self.interaction.hovered() {
            ButtonState::Hovered
        } else {
            ButtonState::Normal
        }
    }

    fn themed_default_style() -> ButtonStyle {
        let theme = current_theme();
        ButtonStyle {
            background: theme.colors.primary,
            background_hovered: theme.colors.primary_hovered,
            background_pressed: theme.colors.primary_pressed,
            background_disabled: theme.colors.disabled,
            text_color: Color::WHITE,
            text_color_disabled: theme.colors.text_muted,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            corner_radius: theme.radii.md,
            padding_h: theme.controls.control_padding_x,
            padding_v: theme.controls.control_padding_y,
            font_size: theme.typography.button_size,
            min_width: 0.0,
            min_height: theme.controls.control_height,
        }
    }

    fn resolved_style(&self) -> ButtonStyle {
        let mut style = self
            .style_override
            .clone()
            .unwrap_or_else(Self::themed_default_style);

        if let Some(background) = self.background_override {
            style.background = background;
        }
        if let Some(text_color) = self.text_color_override {
            style.text_color = text_color;
        }
        if let Some(corner_radius) = self.corner_radius_override {
            style.corner_radius = corner_radius;
        }

        if style.min_width <= 0.0 {
            let char_width = style.font_size * 0.6;
            let estimated_text_width = self.label.len() as f32 * char_width;
            style.min_width = estimated_text_width + style.padding_h * 2.0;
        }
        if style.min_height <= 0.0 {
            style.min_height = current_theme().controls.control_height;
        }

        style
    }

    fn current_background(&self, style: &ButtonStyle) -> Color {
        match self.state() {
            ButtonState::Normal => style.background,
            ButtonState::Hovered => style.background_hovered,
            ButtonState::Pressed => style.background_pressed,
            ButtonState::Disabled => style.background_disabled,
        }
    }

    fn current_text_color(&self, style: &ButtonStyle) -> Color {
        match self.state() {
            ButtonState::Disabled => style.text_color_disabled,
            _ => style.text_color,
        }
    }
}

impl Widget for Button {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        let style = self.resolved_style();
        Style {
            min_size: Size {
                width: length(style.min_width),
                height: length(style.min_height),
            },
            padding: Rect {
                left: length(style.padding_h),
                right: length(style.padding_h),
                top: length(style.padding_v),
                bottom: length(style.padding_v),
            },
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let style = self.resolved_style();
        let bounds = ctx.bounds();
        let bg = self.current_background(&style);
        let text_color = self.current_text_color(&style);
        let scale = ctx.scale_factor;

        // Draw button background
        if style.border_width > 0.0 {
            ctx.fill_bordered_rect(
                bounds,
                bg,
                style.corner_radius,
                style.border_width,
                style.border_color,
            );
        } else {
            ctx.fill_rounded_rect(bounds, bg, style.corner_radius);
        }

        // Focus ring (scale offset for HiDPI)
        if ctx.has_focus() && !self.disabled {
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

        // Draw the button label text, centered
        let text_style = TextStyle::default()
            .with_family(current_theme().typography.font_family)
            .with_size(style.font_size)
            .with_color(text_color);
        ctx.draw_text_centered(&self.label, &text_style, bounds);
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        if self.disabled {
            return;
        }

        match event {
            InputEvent::PointerMove { pos }
                if self.interaction.pointer_move(ctx.contains(*pos)) =>
            {
                ctx.request_paint();
            }
            InputEvent::PointerDown { pos, .. }
                if self.interaction.pointer_down(ctx.contains(*pos)) =>
            {
                ctx.capture_pointer();
            }
            InputEvent::PointerUp { pos, .. } if self.interaction.pressed() => {
                let should_click = self.interaction.pointer_up(ctx.contains(*pos));
                ctx.release_pointer();
                if should_click {
                    if let Some(handler) = &mut self.on_click {
                        handler();
                    }
                }
            }
            InputEvent::KeyDown { .. } if ctx.has_focus() => {
                use sparsha_input::{ActionMapper, StandardAction};
                let mapper = ActionMapper::new();
                if mapper.is_action(event, StandardAction::Activate) {
                    if let Some(handler) = &mut self.on_click {
                        handler();
                    }
                    ctx.stop_propagation();
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let resolved = self.resolved_style();
        let style = TextStyle::default()
            .with_family(current_theme().typography.font_family)
            .with_size(resolved.font_size);
        let (w, h) = ctx.text.measure(&self.label, &style, None);
        Some((w + resolved.padding_h * 2.0, h + resolved.padding_v * 2.0))
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        Some(
            AccessibilityInfo::new(AccessibilityRole::Button)
                .label(self.label.clone())
                .disabled(self.disabled)
                .action(AccessibilityAction::Focus)
                .action(AccessibilityAction::Click),
        )
    }

    fn handle_accessibility_action(
        &mut self,
        action: AccessibilityAction,
        _value: Option<String>,
    ) -> bool {
        if self.disabled {
            return false;
        }

        if matches!(action, AccessibilityAction::Click) {
            if let Some(handler) = &mut self.on_click {
                handler();
            }
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::test_helpers::{
        layout_bounds, mock_event_context, pointer_down_at, pointer_move_at, pointer_up_at,
    };
    use crate::{set_current_theme, Theme};
    use sparsha_input::{FocusManager, InputEvent, Key, KeyboardEvent, NamedKey};
    use sparsha_layout::LayoutTree;

    fn button_bounds() -> (f32, f32, f32, f32) {
        (0.0, 0.0, 100.0, 40.0)
    }

    #[test]
    fn state_transition_pointer_move_inside_then_hovered() {
        let mut button = Button::new("OK");
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        button.set_id(Default::default());

        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, button.id(), false);
        let inside = (x + w / 2.0, y + h / 2.0);
        button.event(&mut ctx, &pointer_move_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Hovered);
    }

    #[test]
    fn state_transition_pointer_move_outside_then_normal() {
        let mut button = Button::new("OK");
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        button.set_id(Default::default());

        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, button.id(), false);
        button.event(&mut ctx, &pointer_move_at(x + w / 2.0, y + h / 2.0));
        assert_eq!(button.state(), ButtonState::Hovered);
        ctx.commands = Default::default();
        button.event(&mut ctx, &pointer_move_at(-10.0, -10.0));
        assert_eq!(button.state(), ButtonState::Normal);
    }

    #[test]
    fn click_flow_down_inside_capture_then_up_fires_click() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = Arc::clone(&clicked);
        let mut button =
            Button::new("OK").on_click(move || clicked_clone.store(true, Ordering::SeqCst));
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        button.set_id(Default::default());

        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, button.id(), false);
        let inside = (x + w / 2.0, y + h / 2.0);

        button.event(&mut ctx, &pointer_down_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Pressed);
        assert!(ctx.commands.capture_pointer);
        assert!(ctx.commands.request_paint);
        assert!(ctx.commands.stop_propagation);

        ctx.commands = Default::default();
        button.event(&mut ctx, &pointer_up_at(inside.0, inside.1));
        assert!(clicked.load(Ordering::SeqCst));
        assert_eq!(button.state(), ButtonState::Hovered);
        assert!(ctx.commands.release_pointer);
        assert!(ctx.commands.request_paint);
    }

    #[test]
    fn disabled_button_ignores_events() {
        let mut button = Button::new("OK").disabled(true);
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        button.set_id(Default::default());

        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, button.id(), false);
        let inside = (x + w / 2.0, y + h / 2.0);
        button.event(&mut ctx, &pointer_move_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Disabled);
        assert_eq!(ctx.commands, Default::default());

        button.event(&mut ctx, &pointer_down_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Disabled);
        assert_eq!(ctx.commands, Default::default());
    }

    #[test]
    fn keyboard_activate_with_focus_fires_click() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = Arc::clone(&clicked);
        let mut button =
            Button::new("OK").on_click(move || clicked_clone.store(true, Ordering::SeqCst));
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(button.id());

        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, button.id(), false);
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::Enter),
                ..Default::default()
            },
        };
        button.event(&mut ctx, &event);
        assert!(clicked.load(Ordering::SeqCst));
        assert!(ctx.commands.stop_propagation);
    }

    #[test]
    fn button_defaults_follow_theme() {
        let mut theme = Theme::default();
        theme.colors.primary = Color::from_hex(0x10B981);
        theme.typography.button_size = 18.0;
        set_current_theme(theme.clone());

        let button = Button::new("Theme");
        let style = button.resolved_style();
        assert_eq!(style.background, theme.colors.primary);
        assert_eq!(style.font_size, 18.0);
    }

    #[test]
    fn button_explicit_background_wins_over_theme() {
        let mut theme = Theme::default();
        theme.colors.primary = Color::from_hex(0x3B82F6);
        set_current_theme(theme);

        let button = Button::new("Override").background(Color::from_hex(0xEF4444));
        let style = button.resolved_style();
        assert_eq!(style.background, Color::from_hex(0xEF4444));
    }
}
