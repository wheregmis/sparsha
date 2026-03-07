//! Button widget.

use crate::{EventContext, EventResponse, PaintContext, Widget};
use spark_core::Color;
use spark_input::InputEvent;
use spark_layout::WidgetId;
use spark_text::TextStyle;
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
            background: Color::from_hex(0x3B82F6),         // Blue
            background_hovered: Color::from_hex(0x2563EB), // Darker blue
            background_pressed: Color::from_hex(0x1D4ED8), // Even darker
            background_disabled: Color::from_hex(0x9CA3AF), // Gray
            text_color: Color::WHITE,
            text_color_disabled: Color::from_hex(0x6B7280),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            corner_radius: 6.0,
            padding_h: 16.0,
            padding_v: 8.0,
            font_size: 14.0,
            min_width: 0.0,  // Will be set based on label
            min_height: 0.0, // Will be set based on font_size
        }
    }
}

/// A clickable button widget.
pub struct Button {
    id: WidgetId,
    label: String,
    style: ButtonStyle,
    state: ButtonState,
    on_click: Option<Box<dyn FnMut() + Send + Sync>>,
}

impl Button {
    /// Create a new button with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        let style = ButtonStyle::default();
        
        // Estimate minimum size based on label and style
        // Rough estimate: ~8px per character for 14px font, plus padding
        let char_width = style.font_size * 0.6;
        let estimated_text_width = label.len() as f32 * char_width;
        let min_width = estimated_text_width + style.padding_h * 2.0;
        
        // Height: font size * line height (~1.4) + vertical padding
        let min_height = style.font_size * 1.4 + style.padding_v * 2.0;
        
        Self {
            id: WidgetId::default(),
            label,
            style: ButtonStyle {
                min_width,
                min_height,
                ..style
            },
            state: ButtonState::Normal,
            on_click: None,
        }
    }

    /// Set the click handler.
    pub fn on_click(mut self, handler: impl FnMut() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// Set the button style.
    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = color;
        self
    }

    /// Set the text color.
    pub fn text_color(mut self, color: Color) -> Self {
        self.style.text_color = color;
        self
    }

    /// Set corner radius.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.style.corner_radius = radius;
        self
    }

    /// Disable the button.
    pub fn disabled(mut self, disabled: bool) -> Self {
        if disabled {
            self.state = ButtonState::Disabled;
        }
        self
    }

    /// Current visual state (for tests and debugging).
    pub fn state(&self) -> ButtonState {
        self.state
    }

    fn current_background(&self) -> Color {
        match self.state {
            ButtonState::Normal => self.style.background,
            ButtonState::Hovered => self.style.background_hovered,
            ButtonState::Pressed => self.style.background_pressed,
            ButtonState::Disabled => self.style.background_disabled,
        }
    }

    fn current_text_color(&self) -> Color {
        match self.state {
            ButtonState::Disabled => self.style.text_color_disabled,
            _ => self.style.text_color,
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
        Style {
            min_size: Size {
                width: length(self.style.min_width),
                height: length(self.style.min_height),
            },
            padding: Rect {
                left: length(self.style.padding_h),
                right: length(self.style.padding_h),
                top: length(self.style.padding_v),
                bottom: length(self.style.padding_v),
            },
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let bg = self.current_background();
        let text_color = self.current_text_color();
        let scale = ctx.scale_factor;

        // Draw button background
        if self.style.border_width > 0.0 {
            ctx.fill_bordered_rect(
                bounds,
                bg,
                self.style.corner_radius,
                self.style.border_width,
                self.style.border_color,
            );
        } else {
            ctx.fill_rounded_rect(bounds, bg, self.style.corner_radius);
        }

        // Focus ring (scale offset for HiDPI)
        if ctx.has_focus() {
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
                Color::from_hex(0x60A5FA),
            );
        }

        // Draw the button label text, centered
        let text_style = TextStyle::default()
            .with_size(self.style.font_size)
            .with_color(text_color);
        ctx.draw_text_centered(&self.label, &text_style, bounds);
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) -> EventResponse {
        if self.state == ButtonState::Disabled {
            return EventResponse::default();
        }

        match event {
            InputEvent::PointerMove { pos } => {
                if ctx.contains(*pos) {
                    if self.state != ButtonState::Pressed {
                        self.state = ButtonState::Hovered;
                    }
                } else {
                    self.state = ButtonState::Normal;
                }
                EventResponse {
                    repaint: true,
                    ..Default::default()
                }
            }
            InputEvent::PointerDown { pos, .. } => {
                if ctx.contains(*pos) {
                    self.state = ButtonState::Pressed;
                    return EventResponse::capture();
                }
                EventResponse::default()
            }
            InputEvent::PointerUp { pos, .. } => {
                if self.state == ButtonState::Pressed {
                    if ctx.contains(*pos) {
                        // Fire click handler
                        if let Some(handler) = &mut self.on_click {
                            handler();
                        }
                        self.state = ButtonState::Hovered;
                    } else {
                        self.state = ButtonState::Normal;
                    }
                    return EventResponse::release();
                }
                EventResponse::default()
            }
            InputEvent::KeyDown { .. } => {
                if ctx.has_focus() {
                    use spark_input::{ActionMapper, StandardAction};
                    let mapper = ActionMapper::new();
                    if mapper.is_action(event, StandardAction::Activate) {
                        if let Some(handler) = &mut self.on_click {
                            handler();
                        }
                        return EventResponse::handled();
                    }
                }
                EventResponse::default()
            }
            _ => EventResponse::default(),
        }
    }

    fn focusable(&self) -> bool {
        self.state != ButtonState::Disabled
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let style = TextStyle::default().with_size(self.style.font_size);
        let (w, h) = ctx.text.measure(&self.label, &style, None);
        Some((
            w + self.style.padding_h * 2.0,
            h + self.style.padding_v * 2.0,
        ))
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
    use spark_input::{FocusManager, InputEvent, Key, KeyboardEvent, NamedKey};
    use spark_layout::LayoutTree;

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

        let mut ctx = mock_event_context(
            layout,
            &layout_tree,
            &mut focus,
            button.id(),
            false,
        );
        let inside = (x + w / 2.0, y + h / 2.0);
        let _ = button.event(&mut ctx, &pointer_move_at(inside.0, inside.1));
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

        let mut ctx = mock_event_context(
            layout,
            &layout_tree,
            &mut focus,
            button.id(),
            false,
        );
        let _ = button.event(&mut ctx, &pointer_move_at(x + w / 2.0, y + h / 2.0));
        assert_eq!(button.state(), ButtonState::Hovered);
        let _ = button.event(&mut ctx, &pointer_move_at(-10.0, -10.0));
        assert_eq!(button.state(), ButtonState::Normal);
    }

    #[test]
    fn click_flow_down_inside_capture_then_up_fires_click() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = Arc::clone(&clicked);
        let mut button = Button::new("OK").on_click(move || clicked_clone.store(true, Ordering::SeqCst));
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        button.set_id(Default::default());

        let mut ctx = mock_event_context(
            layout,
            &layout_tree,
            &mut focus,
            button.id(),
            false,
        );
        let inside = (x + w / 2.0, y + h / 2.0);

        let r = button.event(&mut ctx, &pointer_down_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Pressed);
        crate::assert_event_response!(r, handled: true, repaint: true, capture_pointer: true);

        let r = button.event(&mut ctx, &pointer_up_at(inside.0, inside.1));
        assert!(clicked.load(Ordering::SeqCst));
        assert_eq!(button.state(), ButtonState::Hovered);
        crate::assert_event_response!(r, handled: true, repaint: true, release_pointer: true);
    }

    #[test]
    fn disabled_button_ignores_events() {
        let mut button = Button::new("OK").disabled(true);
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        button.set_id(Default::default());

        let mut ctx = mock_event_context(
            layout,
            &layout_tree,
            &mut focus,
            button.id(),
            false,
        );
        let inside = (x + w / 2.0, y + h / 2.0);
        let r = button.event(&mut ctx, &pointer_move_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Disabled);
        assert!(!r.handled && !r.repaint);

        let r = button.event(&mut ctx, &pointer_down_at(inside.0, inside.1));
        assert_eq!(button.state(), ButtonState::Disabled);
        assert!(!r.handled);
    }

    #[test]
    fn keyboard_activate_with_focus_fires_click() {
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = Arc::clone(&clicked);
        let mut button = Button::new("OK").on_click(move || clicked_clone.store(true, Ordering::SeqCst));
        let (x, y, w, h) = button_bounds();
        let layout = layout_bounds(x, y, w, h);
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        focus.set_focus(button.id());

        let mut ctx = mock_event_context(
            layout,
            &layout_tree,
            &mut focus,
            button.id(),
            false,
        );
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::Enter),
                ..Default::default()
            },
        };
        let r = button.event(&mut ctx, &event);
        assert!(clicked.load(Ordering::SeqCst));
        assert!(r.handled);
    }
}

