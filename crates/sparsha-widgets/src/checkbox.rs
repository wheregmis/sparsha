//! Checkbox widget.

use crate::{
    control_state::{focus_ring_border_width, focus_ring_bounds, focus_ring_color, ControlState},
    current_theme, responsive_theme_controls, AccessibilityAction, AccessibilityInfo,
    AccessibilityRole, EventContext, PaintContext, Widget,
};
use bon::bon;
use sparsha_core::Color;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
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
    declared_checked: bool,
    disabled: bool,
    interaction: ControlState,
    style: CheckboxStyle,
    use_theme_defaults: bool,
    size_override: Option<f32>,
    on_toggle: Option<Box<dyn FnMut(bool)>>,
}

#[derive(Clone, Copy)]
struct CheckboxBuildState {
    checked: bool,
    declared_checked: bool,
}

impl Checkbox {
    fn unchecked() -> Self {
        Self {
            id: WidgetId::default(),
            checked: false,
            declared_checked: false,
            disabled: false,
            interaction: ControlState::default(),
            style: CheckboxStyle::default(),
            use_theme_defaults: true,
            size_override: None,
            on_toggle: None,
        }
    }

    fn style_override(mut self, style: CheckboxStyle) -> Self {
        self.style = style;
        self.use_theme_defaults = false;
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

    fn themed_default_style() -> CheckboxStyle {
        let theme = current_theme();
        let controls = responsive_theme_controls(&theme);
        CheckboxStyle {
            size: controls.checkbox_size,
            corner_radius: theme.radii.sm,
            border_width: 1.0,
            background: theme.colors.surface,
            background_hovered: theme.colors.background,
            background_checked: theme.colors.primary,
            background_disabled: theme.colors.disabled,
            border_color: theme.colors.border,
            border_color_checked: theme.colors.primary_hovered,
            border_color_disabled: theme.colors.border,
            mark_color: Color::WHITE,
            focus_color: theme.colors.border_focus,
        }
    }

    fn resolved_style(&self) -> CheckboxStyle {
        let mut style = if self.use_theme_defaults {
            Self::themed_default_style()
        } else {
            self.style.clone()
        };
        if let Some(size) = self.size_override {
            style.size = size;
        }
        style
    }
}

#[bon]
impl Checkbox {
    #[builder(
        start_fn(name = builder, vis = "pub"),
        finish_fn(name = build, vis = "pub"),
        builder_type(name = CheckboxBuilder, vis = "pub"),
        state_mod(vis = "pub")
    )]
    fn builder_init(
        #[builder(default)] checked: bool,
        #[builder(default)] disabled: bool,
        style: Option<CheckboxStyle>,
        size: Option<f32>,
        #[builder(with = |handler: impl FnMut(bool) + 'static| Box::new(handler) as Box<dyn FnMut(bool)>)]
        on_toggle: Option<Box<dyn FnMut(bool)>>,
    ) -> Self {
        let mut checkbox = Self::unchecked();
        checkbox.checked = checked;
        checkbox.declared_checked = checked;
        checkbox.disabled = disabled;
        if let Some(style) = style {
            checkbox = checkbox.style_override(style);
        }
        if let Some(size) = size {
            checkbox.size_override = Some(size.max(1.0));
        }
        if let Some(on_toggle) = on_toggle {
            checkbox.on_toggle = Some(on_toggle);
        }
        checkbox
    }
}

impl Widget for Checkbox {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn rebuild(&mut self, ctx: &mut crate::BuildContext) {
        if let Some(state) = ctx
            .take_boxed_state()
            .and_then(|state| state.downcast::<CheckboxBuildState>().ok())
            .map(|state| *state)
        {
            if state.declared_checked == self.declared_checked {
                self.checked = state.checked;
            }
        }

        ctx.store_boxed_state(Box::new(CheckboxBuildState {
            checked: self.checked,
            declared_checked: self.declared_checked,
        }));
    }

    fn persist_build_state(&self, ctx: &mut crate::BuildContext) {
        ctx.store_boxed_state(Box::new(CheckboxBuildState {
            checked: self.checked,
            declared_checked: self.declared_checked,
        }));
    }

    fn style(&self) -> Style {
        let style = self.resolved_style();
        let size = length(style.size);
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
        let style = self.resolved_style();
        let bounds = ctx.bounds();
        let scale = ctx.scale_factor;

        let (background, border_color) = if self.disabled {
            (style.background_disabled, style.border_color_disabled)
        } else if self.checked {
            (style.background_checked, style.border_color_checked)
        } else if self.interaction.hovered() || self.interaction.pressed() {
            (style.background_hovered, style.border_color)
        } else {
            (style.background, style.border_color)
        };

        ctx.fill_bordered_rect(
            bounds,
            background,
            style.corner_radius,
            style.border_width,
            border_color,
        );

        if self.checked {
            let theme = current_theme();
            let mark_style = sparsha_text::TextStyle::new()
                .with_family(theme.typography.font_family.clone())
                .with_size(style.size * 0.9)
                .with_color(style.mark_color)
                .bold();
            let mark_bounds = sparsha_core::Rect::new(
                bounds.x,
                bounds.y + style.size * scale * 0.03,
                bounds.width,
                bounds.height,
            );
            ctx.draw_text_centered("✓", &mark_style, mark_bounds);
        }

        if ctx.has_focus() && !self.disabled {
            let controls = current_theme().controls;
            let focus_bounds = focus_ring_bounds(bounds, scale, &controls);
            ctx.fill_bordered_rect(
                focus_bounds,
                Color::TRANSPARENT,
                style.corner_radius + 2.0,
                focus_ring_border_width(scale, &controls),
                focus_ring_color(style.focus_color),
            );
        }
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
                let should_toggle = self.interaction.pointer_up(ctx.contains(*pos));
                if should_toggle {
                    self.toggle();
                }
                ctx.release_pointer();
            }
            InputEvent::KeyDown { .. } if ctx.has_focus() => {
                use sparsha_input::{ActionMapper, StandardAction};
                let mapper = ActionMapper::new();
                if mapper.is_action(event, StandardAction::Activate) {
                    self.toggle();
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

    fn measure(&self, _ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        let style = self.resolved_style();
        Some((style.size, style.size))
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        Some(
            AccessibilityInfo::new(AccessibilityRole::CheckBox)
                .checked(self.checked)
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
            self.toggle();
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{layout_bounds, mock_event_context, pointer_down_at, pointer_up_at};
    use crate::{set_current_theme, set_current_viewport, Theme, ViewportInfo};
    use sparsha_input::{FocusManager, Key, KeyboardEvent, NamedKey};
    use sparsha_layout::LayoutTree;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    fn checkbox_env() -> (LayoutTree, FocusManager, sparsha_layout::ComputedLayout) {
        let layout_tree = LayoutTree::new();
        let focus = FocusManager::new();
        let layout = layout_bounds(0.0, 0.0, 20.0, 20.0);
        (layout_tree, focus, layout)
    }

    #[test]
    fn pointer_toggle_invokes_callback() {
        let toggled = Arc::new(AtomicBool::new(false));
        let toggled_cb = Arc::clone(&toggled);
        let mut checkbox = Checkbox::builder()
            .on_toggle(move |checked| toggled_cb.store(checked, Ordering::SeqCst))
            .build();
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
        let mut checkbox = Checkbox::builder().build();
        checkbox.set_id(Default::default());
        let (layout_tree, mut focus, layout) = checkbox_env();
        focus.set_focus(checkbox.id());
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, checkbox.id(), false);

        let event = sparsha_input::InputEvent::KeyDown {
            event: KeyboardEvent::key_down(
                Key::Named(NamedKey::Enter),
                sparsha_input::ui_events::keyboard::Code::Unidentified,
            ),
        };
        checkbox.event(&mut ctx, &event);
        assert!(ctx.commands.stop_propagation);
        assert!(checkbox.is_checked());
    }

    #[test]
    fn disabled_checkbox_ignores_events() {
        let mut checkbox = Checkbox::builder().disabled(true).build();
        checkbox.set_id(Default::default());
        let (layout_tree, mut focus, layout) = checkbox_env();
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, checkbox.id(), false);

        checkbox.event(&mut ctx, &pointer_down_at(10.0, 10.0));
        checkbox.event(&mut ctx, &pointer_up_at(10.0, 10.0));
        assert!(!checkbox.is_checked());
    }

    #[test]
    fn themed_defaults_scale_down_for_mobile_viewport() {
        let mut theme = Theme::default();
        theme.controls.checkbox_size = 18.0;
        set_current_theme(theme);
        set_current_viewport(ViewportInfo::new(390.0, 844.0));

        let checkbox = Checkbox::builder().build();
        let style = checkbox.resolved_style();
        assert!(style.size < 18.0);
    }

    #[test]
    fn builder_sets_explicit_configuration() {
        let checkbox = Checkbox::builder()
            .checked(true)
            .disabled(true)
            .size(24.0)
            .build();

        assert!(checkbox.checked);
        assert!(checkbox.declared_checked);
        assert!(checkbox.disabled);
        assert_eq!(checkbox.size_override, Some(24.0));
    }
}
