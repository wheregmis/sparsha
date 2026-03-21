//! Checkbox widget.

use crate::{EventContext, PaintContext, Widget};
use sparsh_core::Color;
use sparsh_input::InputEvent;
use sparsh_layout::WidgetId;
use taffy::prelude::*;

/// Style configuration for a checkbox.
#[derive(Clone, Debug)]
pub struct CheckboxStyle {
    pub size: f32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub background: Color,
    pub background_hovered: Color,
    pub background_checked: Color,
    pub background_disabled: Color,
    pub border_color: Color,
    pub border_color_checked: Color,
    pub border_color_disabled: Color,
    pub mark_color: Color,
    pub focus_color: Color,
}

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self {
            size: 18.0,
            corner_radius: 4.0,
            border_width: 1.0,
            background: Color::WHITE,
            background_hovered: Color::from_hex(0xF3F4F6),
            background_checked: Color::from_hex(0x3B82F6),
            background_disabled: Color::from_hex(0xE5E7EB),
            border_color: Color::from_hex(0x9CA3AF),
            border_color_checked: Color::from_hex(0x2563EB),
            border_color_disabled: Color::from_hex(0xD1D5DB),
            mark_color: Color::WHITE,
            focus_color: Color::from_hex(0x60A5FA),
        }
    }
}

/// A focusable, toggleable checkbox widget.
pub struct Checkbox {
    id: WidgetId,
    checked: bool,
    disabled: bool,
    hovered: bool,
    pressed: bool,
    style: CheckboxStyle,
    on_toggle: Option<Box<dyn FnMut(bool)>>,
}

impl Checkbox {
    /// Create a new unchecked checkbox.
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            checked: false,
            disabled: false,
            hovered: false,
            pressed: false,
            style: CheckboxStyle::default(),
            on_toggle: None,
        }
    }

    /// Create a checkbox with an initial checked state.
    pub fn with_checked(checked: bool) -> Self {
        Self::new().checked(checked)
    }

    /// Set checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set style.
    pub fn with_style(mut self, style: CheckboxStyle) -> Self {
        self.style = style;
        self
    }

    /// Set checkbox square size in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.style.size = size.max(1.0);
        self
    }

    /// Set toggle callback.
    pub fn on_toggle(mut self, handler: impl FnMut(bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }

    /// Get current checked state.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    fn toggle(&mut self) {
        self.checked = !self.checked;
        if let Some(handler) = &mut self.on_toggle {
            handler(self.checked);
        }
    }
}

impl Default for Checkbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Checkbox {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        let size = length(self.style.size);
        Style {
            size: Size {
                width: size,
                height: size,
            },
            min_size: Size {
                width: size,
                height: size,
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let scale = ctx.scale_factor;

        let (background, border_color) = if self.disabled {
            (
                self.style.background_disabled,
                self.style.border_color_disabled,
            )
        } else if self.checked {
            (
                self.style.background_checked,
                self.style.border_color_checked,
            )
        } else if self.hovered || self.pressed {
            (self.style.background_hovered, self.style.border_color)
        } else {
            (self.style.background, self.style.border_color)
        };

        ctx.fill_bordered_rect(
            bounds,
            background,
            self.style.corner_radius,
            self.style.border_width,
            border_color,
        );

        if self.checked {
            // Simple square check mark for now.
            let mark_inset = (4.0 * scale).max(2.0);
            let mark_bounds = sparsh_core::Rect::new(
                bounds.x + mark_inset,
                bounds.y + mark_inset,
                (bounds.width - mark_inset * 2.0).max(1.0),
                (bounds.height - mark_inset * 2.0).max(1.0),
            );
            ctx.fill_rounded_rect(mark_bounds, self.style.mark_color, 2.0);
        }

        if ctx.has_focus() && !self.disabled {
            let offset = 2.0 * scale;
            let focus_bounds = sparsh_core::Rect::new(
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
                self.style.focus_color,
            );
        }
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        if self.disabled {
            return;
        }

        match event {
            InputEvent::PointerMove { pos } => {
                let hovered = ctx.contains(*pos);
                if self.hovered != hovered {
                    self.hovered = hovered;
                    ctx.request_paint();
                }
            }
            InputEvent::PointerDown { pos, .. } => {
                if ctx.contains(*pos) {
                    self.pressed = true;
                    ctx.capture_pointer();
                }
            }
            InputEvent::PointerUp { pos, .. } => {
                if self.pressed {
                    self.pressed = false;
                    if ctx.contains(*pos) {
                        self.toggle();
                        ctx.release_pointer();
                    } else {
                        ctx.release_pointer();
                    }
                }
            }
            InputEvent::KeyDown { .. } => {
                if ctx.has_focus() {
                    use sparsh_input::{ActionMapper, StandardAction};
                    let mapper = ActionMapper::new();
                    if mapper.is_action(event, StandardAction::Activate) {
                        self.toggle();
                        ctx.stop_propagation();
                        ctx.request_paint();
                    }
                }
            }
            _ => {}
        }
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }

    fn measure(&self, _ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        Some((self.style.size, self.style.size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{layout_bounds, mock_event_context, pointer_down_at, pointer_up_at};
    use sparsh_input::{FocusManager, Key, KeyboardEvent, NamedKey};
    use sparsh_layout::LayoutTree;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    fn checkbox_env() -> (LayoutTree, FocusManager, sparsh_layout::ComputedLayout) {
        let layout_tree = LayoutTree::new();
        let focus = FocusManager::new();
        let layout = layout_bounds(0.0, 0.0, 20.0, 20.0);
        (layout_tree, focus, layout)
    }

    #[test]
    fn pointer_toggle_invokes_callback() {
        let toggled = Arc::new(AtomicBool::new(false));
        let toggled_cb = Arc::clone(&toggled);
        let mut checkbox =
            Checkbox::new().on_toggle(move |checked| toggled_cb.store(checked, Ordering::SeqCst));
        checkbox.set_id(Default::default());

        let (layout_tree, mut focus, layout) = checkbox_env();
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, checkbox.id(), false);

        checkbox.event(&mut ctx, &pointer_down_at(10.0, 10.0));
        assert!(ctx.commands.capture_pointer);

        ctx.commands = Default::default();
        checkbox.event(&mut ctx, &pointer_up_at(10.0, 10.0));
        assert!(ctx.commands.release_pointer);
        assert!(checkbox.is_checked());
        assert!(toggled.load(Ordering::SeqCst));
    }

    #[test]
    fn keyboard_activate_toggles_when_focused() {
        let mut checkbox = Checkbox::new();
        checkbox.set_id(Default::default());
        let (layout_tree, mut focus, layout) = checkbox_env();
        focus.set_focus(checkbox.id());
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, checkbox.id(), false);

        let event = sparsh_input::InputEvent::KeyDown {
            event: KeyboardEvent::key_down(
                Key::Named(NamedKey::Enter),
                sparsh_input::ui_events::keyboard::Code::Unidentified,
            ),
        };
        checkbox.event(&mut ctx, &event);
        assert!(ctx.commands.stop_propagation);
        assert!(checkbox.is_checked());
    }

    #[test]
    fn disabled_checkbox_ignores_events() {
        let mut checkbox = Checkbox::new().disabled(true);
        checkbox.set_id(Default::default());
        let (layout_tree, mut focus, layout) = checkbox_env();
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, checkbox.id(), false);

        checkbox.event(&mut ctx, &pointer_down_at(10.0, 10.0));
        checkbox.event(&mut ctx, &pointer_up_at(10.0, 10.0));
        assert!(!checkbox.is_checked());
    }
}
