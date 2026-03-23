use sparsha_core::{Color, Rect};

use crate::{current_viewport, ThemeControls, ViewportClass};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ControlState {
    hovered: bool,
    pressed: bool,
}

impl ControlState {
    pub(crate) fn hovered(&self) -> bool {
        self.hovered
    }

    pub(crate) fn pressed(&self) -> bool {
        self.pressed
    }

    pub(crate) fn pointer_move(&mut self, contains: bool) -> bool {
        let previous = self.hovered;
        self.hovered = contains;
        previous != self.hovered
    }

    pub(crate) fn pointer_down(&mut self, contains: bool) -> bool {
        if !contains {
            return false;
        }
        let previous = self.pressed;
        self.hovered = true;
        self.pressed = true;
        previous != self.pressed
    }

    pub(crate) fn pointer_up(&mut self, contains: bool) -> bool {
        let was_pressed = self.pressed;
        self.pressed = false;
        self.hovered = contains;
        was_pressed && contains
    }
}

pub(crate) fn focus_ring_bounds(bounds: Rect, scale_factor: f32, controls: &ThemeControls) -> Rect {
    let ring_width = resolved_focus_ring_width(controls) * scale_factor;
    Rect::new(
        bounds.x - ring_width,
        bounds.y - ring_width,
        bounds.width + ring_width * 2.0,
        bounds.height + ring_width * 2.0,
    )
}

pub(crate) fn focus_ring_border_width(scale_factor: f32, controls: &ThemeControls) -> f32 {
    resolved_focus_ring_width(controls) * scale_factor
}

pub(crate) fn focus_ring_color(color: Color) -> Color {
    color.with_alpha(0.5)
}

fn resolved_focus_ring_width(controls: &ThemeControls) -> f32 {
    match current_viewport().class {
        ViewportClass::Desktop => controls.focus_ring_width,
        ViewportClass::Tablet => (controls.focus_ring_width - 0.25).max(1.5),
        ViewportClass::Mobile => (controls.focus_ring_width - 0.5).max(1.5),
    }
}
