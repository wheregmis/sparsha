//! Viewport context and responsive metrics.

use crate::{Theme, ThemeControls};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewportClass {
    #[default]
    Desktop,
    Tablet,
    Mobile,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewportOrientation {
    #[default]
    Landscape,
    Portrait,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportInfo {
    pub width: f32,
    pub height: f32,
    pub shortest_side: f32,
    pub orientation: ViewportOrientation,
    pub class: ViewportClass,
}

impl ViewportInfo {
    pub fn new(width: f32, height: f32) -> Self {
        let width = width.max(0.0);
        let height = height.max(0.0);
        let shortest_side = width.min(height);
        let orientation = if height > width {
            ViewportOrientation::Portrait
        } else {
            ViewportOrientation::Landscape
        };
        let class = classify_viewport_width(width);
        Self {
            width,
            height,
            shortest_side,
            orientation,
            class,
        }
    }
}

impl Default for ViewportInfo {
    fn default() -> Self {
        Self::new(1024.0, 768.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResponsiveTypography {
    pub body_size: f32,
    pub small_size: f32,
    pub title_size: f32,
    pub subheader_size: f32,
    pub button_size: f32,
}

thread_local! {
    static CURRENT_VIEWPORT: RefCell<ViewportInfo> = RefCell::new(ViewportInfo::default());
}

pub fn current_viewport() -> ViewportInfo {
    CURRENT_VIEWPORT.with(|slot| *slot.borrow())
}

#[doc(hidden)]
pub fn set_current_viewport(viewport: ViewportInfo) {
    CURRENT_VIEWPORT.with(|slot| {
        *slot.borrow_mut() = viewport;
    });
}

pub(crate) fn classify_viewport_width(width: f32) -> ViewportClass {
    if width < 768.0 {
        ViewportClass::Mobile
    } else if width < 1024.0 {
        ViewportClass::Tablet
    } else {
        ViewportClass::Desktop
    }
}

pub(crate) fn responsive_typography(theme: &Theme) -> ResponsiveTypography {
    let class = current_viewport().class;
    let body_size = match class {
        ViewportClass::Desktop => theme.typography.body_size,
        ViewportClass::Tablet => (theme.typography.body_size - 1.0).max(13.0),
        ViewportClass::Mobile => (theme.typography.body_size - 2.0).max(13.0),
    };
    let small_size = match class {
        ViewportClass::Desktop => theme.typography.small_size,
        ViewportClass::Tablet => (theme.typography.small_size - 1.0).max(11.0),
        ViewportClass::Mobile => (theme.typography.small_size - 1.0).max(10.0),
    };
    let title_size = match class {
        ViewportClass::Desktop => theme.typography.title_size,
        ViewportClass::Tablet => (theme.typography.title_size - 2.0).max(22.0),
        ViewportClass::Mobile => (theme.typography.title_size - 4.0).max(20.0),
    };
    let button_size = match class {
        ViewportClass::Desktop => theme.typography.button_size,
        ViewportClass::Tablet => (theme.typography.button_size - 1.0).max(13.0),
        ViewportClass::Mobile => (theme.typography.button_size - 2.0).max(12.0),
    };
    let subheader_size = (title_size - 6.0).max(body_size + 2.0);

    ResponsiveTypography {
        body_size,
        small_size,
        title_size,
        subheader_size,
        button_size,
    }
}

pub(crate) fn responsive_theme_controls(theme: &Theme) -> ThemeControls {
    let mut controls = theme.controls;
    match current_viewport().class {
        ViewportClass::Desktop => {}
        ViewportClass::Tablet => {
            controls.control_height = (controls.control_height - 2.0).max(34.0);
            controls.control_padding_x = (controls.control_padding_x - 1.0).max(10.0);
            controls.control_padding_y = (controls.control_padding_y - 1.0).max(7.0);
            controls.focus_ring_width = (controls.focus_ring_width - 0.25).max(1.5);
            controls.checkbox_size = (controls.checkbox_size - 1.0).max(16.0);
            controls.scrollbar_thickness = (controls.scrollbar_thickness - 2.0).max(8.0);
        }
        ViewportClass::Mobile => {
            controls.control_height = (controls.control_height - 4.0).max(32.0);
            controls.control_padding_x = (controls.control_padding_x - 2.0).max(10.0);
            controls.control_padding_y = (controls.control_padding_y - 2.0).max(6.0);
            controls.focus_ring_width = (controls.focus_ring_width - 0.5).max(1.5);
            controls.checkbox_size = (controls.checkbox_size - 2.0).max(15.0);
            controls.scrollbar_thickness = (controls.scrollbar_thickness - 3.0).max(7.0);
        }
    }
    controls
}

pub(crate) fn responsive_text_area_min_height(theme: &Theme) -> f32 {
    let min_height: f32 = match current_viewport().class {
        ViewportClass::Desktop => 96.0,
        ViewportClass::Tablet => 88.0,
        ViewportClass::Mobile => 80.0,
    };
    min_height.max(responsive_theme_controls(theme).control_height * 2.2_f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_classification_uses_logical_width_breakpoints() {
        assert_eq!(ViewportInfo::new(390.0, 844.0).class, ViewportClass::Mobile);
        assert_eq!(
            ViewportInfo::new(820.0, 1180.0).class,
            ViewportClass::Tablet
        );
        assert_eq!(
            ViewportInfo::new(1280.0, 800.0).class,
            ViewportClass::Desktop
        );
    }

    #[test]
    fn viewport_orientation_and_shortest_side_are_derived() {
        let portrait = ViewportInfo::new(768.0, 1024.0);
        assert_eq!(portrait.orientation, ViewportOrientation::Portrait);
        assert_eq!(portrait.shortest_side, 768.0);

        let landscape = ViewportInfo::new(1180.0, 820.0);
        assert_eq!(landscape.orientation, ViewportOrientation::Landscape);
        assert_eq!(landscape.shortest_side, 820.0);
    }
}
