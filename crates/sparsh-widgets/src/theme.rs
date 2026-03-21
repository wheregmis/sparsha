//! Theme tokens and runtime theme context.

use sparsh_core::Color;
use std::cell::RefCell;

/// Top-level theme object.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub colors: ThemeColors,
    pub typography: ThemeTypography,
    pub spacing: ThemeSpacing,
    pub radii: ThemeRadii,
    pub controls: ThemeControls,
}

impl Theme {
    pub fn light() -> Self {
        Self::default()
    }

    pub fn dark() -> Self {
        Self {
            colors: ThemeColors {
                background: Color::from_hex(0x0F172A),
                surface: Color::from_hex(0x111827),
                surface_variant: Color::from_hex(0x1E293B),
                surface_done: Color::from_hex(0x0B1220),
                text_primary: Color::from_hex(0xE2E8F0),
                text_muted: Color::from_hex(0x94A3B8),
                primary: Color::from_hex(0x3B82F6),
                primary_hovered: Color::from_hex(0x2563EB),
                primary_pressed: Color::from_hex(0x1D4ED8),
                error: Color::from_hex(0xB91C1C),
                error_hovered: Color::from_hex(0x991B1B),
                error_pressed: Color::from_hex(0x7F1D1D),
                border: Color::from_hex(0x334155),
                border_focus: Color::from_hex(0x60A5FA),
                disabled: Color::from_hex(0x64748B),
                input_background: Color::from_hex(0x1E293B),
                input_placeholder: Color::from_hex(0x64748B),
            },
            typography: ThemeTypography::default(),
            spacing: ThemeSpacing::default(),
            radii: ThemeRadii::default(),
            controls: ThemeControls::default(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: ThemeColors::default(),
            typography: ThemeTypography::default(),
            spacing: ThemeSpacing::default(),
            radii: ThemeRadii::default(),
            controls: ThemeControls::default(),
        }
    }
}

/// Color tokens used by core widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeColors {
    pub background: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub surface_done: Color,
    pub text_primary: Color,
    pub text_muted: Color,
    pub primary: Color,
    pub primary_hovered: Color,
    pub primary_pressed: Color,
    pub error: Color,
    pub error_hovered: Color,
    pub error_pressed: Color,
    pub border: Color,
    pub border_focus: Color,
    pub disabled: Color,
    pub input_background: Color,
    pub input_placeholder: Color,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            background: Color::from_hex(0xF3F4F6),
            surface: Color::WHITE,
            surface_variant: Color::from_hex(0xF1F5F9),
            surface_done: Color::from_hex(0xE2E8F0),
            text_primary: Color::from_hex(0x1F2937),
            text_muted: Color::from_hex(0x6B7280),
            primary: Color::from_hex(0x3B82F6),
            primary_hovered: Color::from_hex(0x2563EB),
            primary_pressed: Color::from_hex(0x1D4ED8),
            error: Color::from_hex(0xDC2626),
            error_hovered: Color::from_hex(0xB91C1C),
            error_pressed: Color::from_hex(0x991B1B),
            border: Color::from_hex(0xD1D5DB),
            border_focus: Color::from_hex(0x60A5FA),
            disabled: Color::from_hex(0x9CA3AF),
            input_background: Color::WHITE,
            input_placeholder: Color::from_hex(0x9CA3AF),
        }
    }
}

/// Typography tokens used by core widgets.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeTypography {
    pub font_family: String,
    pub body_size: f32,
    pub small_size: f32,
    pub title_size: f32,
    pub button_size: f32,
    pub line_height: f32,
}

impl Default for ThemeTypography {
    fn default() -> Self {
        Self {
            font_family: String::from("Inter"),
            body_size: 16.0,
            small_size: 12.0,
            title_size: 24.0,
            button_size: 14.0,
            line_height: 1.2,
        }
    }
}

/// Spacing tokens used by core widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeSpacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 16.0,
            xl: 24.0,
        }
    }
}

/// Radius tokens used by core widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeRadii {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

impl Default for ThemeRadii {
    fn default() -> Self {
        Self {
            sm: 4.0,
            md: 6.0,
            lg: 12.0,
        }
    }
}

/// Shared control metrics used by the built-in widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeControls {
    pub control_height: f32,
    pub control_padding_x: f32,
    pub control_padding_y: f32,
    pub focus_ring_width: f32,
    pub checkbox_size: f32,
    pub scrollbar_thickness: f32,
}

impl Default for ThemeControls {
    fn default() -> Self {
        Self {
            control_height: 38.0,
            control_padding_x: 12.0,
            control_padding_y: 8.0,
            focus_ring_width: 2.0,
            checkbox_size: 18.0,
            scrollbar_thickness: 10.0,
        }
    }
}

thread_local! {
    static CURRENT_THEME: RefCell<Theme> = RefCell::new(Theme::default());
}

/// Set the current app theme for this thread.
pub fn set_current_theme(theme: Theme) {
    CURRENT_THEME.with(|slot| {
        *slot.borrow_mut() = theme;
    });
}

/// Read the current app theme.
pub fn current_theme() -> Theme {
    CURRENT_THEME.with(|slot| slot.borrow().clone())
}
