//! Input event types - re-exported from ui-events.
//!
//! We use the ui-events crate from the Linebender ecosystem which provides
//! W3C-compliant UI event types with winit integration.

pub use ui_events::{
    keyboard::{CompositionEvent, Key, KeyState, KeyboardEvent, Modifiers, NamedKey},
    pointer::{PointerButton, PointerId, PointerState, PointerType},
    ScrollDelta,
};

use glam::Vec2;

/// Wrapper for common input events used in the widget system.
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// Pointer (mouse/touch/pen) moved.
    PointerMove { pos: Vec2 },
    /// Pointer button pressed.
    PointerDown { pos: Vec2, button: PointerButton },
    /// Pointer button released.
    PointerUp { pos: Vec2, button: PointerButton },
    /// Scroll wheel event.
    Scroll {
        pos: Vec2,
        delta: Vec2,
        modifiers: Modifiers,
    },
    /// Key pressed.
    KeyDown { event: KeyboardEvent },
    /// Key released.
    KeyUp { event: KeyboardEvent },
    /// Text input (after IME processing).
    TextInput { text: String },
    /// Paste content provided by the runtime clipboard bridge.
    Paste { text: String },
    /// IME composition started.
    CompositionStart,
    /// IME composition updated.
    CompositionUpdate { text: String },
    /// IME composition committed or ended.
    CompositionEnd { text: String },
    /// Focus gained.
    FocusGained,
    /// Focus lost.
    FocusLost,
}

impl InputEvent {
    /// Get the position if this is a pointer event.
    pub fn pos(&self) -> Option<Vec2> {
        match self {
            InputEvent::PointerMove { pos, .. } => Some(*pos),
            InputEvent::PointerDown { pos, .. } => Some(*pos),
            InputEvent::PointerUp { pos, .. } => Some(*pos),
            InputEvent::Scroll { pos, .. } => Some(*pos),
            _ => None,
        }
    }

    /// Check if this is a key event.
    pub fn is_key_event(&self) -> bool {
        matches!(self, InputEvent::KeyDown { .. } | InputEvent::KeyUp { .. })
    }

    /// Check if this is a pointer event.
    pub fn is_pointer_event(&self) -> bool {
        matches!(
            self,
            InputEvent::PointerMove { .. }
                | InputEvent::PointerDown { .. }
                | InputEvent::PointerUp { .. }
                | InputEvent::Scroll { .. }
        )
    }

    /// Check if left mouse button is pressed (for PointerDown events).
    pub fn is_left_click(&self) -> bool {
        matches!(
            self,
            InputEvent::PointerDown {
                button: PointerButton::Primary,
                ..
            }
        )
    }

    /// Check if right mouse button is pressed (for PointerDown events).
    pub fn is_right_click(&self) -> bool {
        matches!(
            self,
            InputEvent::PointerDown {
                button: PointerButton::Secondary,
                ..
            }
        )
    }

    /// Get modifiers from keyboard events.
    pub fn modifiers(&self) -> Option<Modifiers> {
        match self {
            InputEvent::KeyDown { event } | InputEvent::KeyUp { event } => Some(event.modifiers),
            _ => None,
        }
    }
}

/// Helper for checking common key combinations.
pub mod shortcuts {
    use super::*;
    use ui_events::keyboard::Modifiers;

    fn is_char(event: &KeyboardEvent, ch: char) -> bool {
        matches!(&event.key, Key::Character(s) if s.starts_with(ch.to_ascii_lowercase()) || s.starts_with(ch.to_ascii_uppercase()))
    }

    /// Return whether the platform-primary shortcut modifier is active.
    ///
    /// Sparsha treats Meta as primary on macOS and wasm targets, and Control elsewhere.
    pub fn primary_modifier(modifiers: Modifiers) -> bool {
        #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
        {
            modifiers.meta()
        }

        #[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
        {
            modifiers.ctrl()
        }
    }

    /// Check if this is the primary copy shortcut.
    pub fn is_copy(event: &KeyboardEvent) -> bool {
        primary_modifier(event.modifiers) && is_char(event, 'c')
    }

    /// Check if this is the primary paste shortcut.
    pub fn is_paste(event: &KeyboardEvent) -> bool {
        primary_modifier(event.modifiers) && is_char(event, 'v')
    }

    /// Check if this is the primary cut shortcut.
    pub fn is_cut(event: &KeyboardEvent) -> bool {
        primary_modifier(event.modifiers) && is_char(event, 'x')
    }

    /// Check if this is the primary select-all shortcut.
    pub fn is_select_all(event: &KeyboardEvent) -> bool {
        primary_modifier(event.modifiers) && is_char(event, 'a')
    }

    /// Check if this is the primary undo shortcut.
    pub fn is_undo(event: &KeyboardEvent) -> bool {
        primary_modifier(event.modifiers) && !event.modifiers.shift() && is_char(event, 'z')
    }

    /// Check if this is the primary redo shortcut.
    pub fn is_redo(event: &KeyboardEvent) -> bool {
        (primary_modifier(event.modifiers) && event.modifiers.shift() && is_char(event, 'z'))
            || (primary_modifier(event.modifiers) && is_char(event, 'y'))
    }

    /// Check if this is the Escape key.
    pub fn is_escape(event: &KeyboardEvent) -> bool {
        matches!(&event.key, Key::Named(NamedKey::Escape))
    }

    /// Check if this is the Enter key.
    pub fn is_enter(event: &KeyboardEvent) -> bool {
        matches!(&event.key, Key::Named(NamedKey::Enter))
    }

    /// Check if this is the Tab key.
    pub fn is_tab(event: &KeyboardEvent) -> bool {
        matches!(&event.key, Key::Named(NamedKey::Tab))
    }

    /// Check if this is Backspace.
    pub fn is_backspace(event: &KeyboardEvent) -> bool {
        matches!(&event.key, Key::Named(NamedKey::Backspace))
    }

    /// Check if this is Delete.
    pub fn is_delete(event: &KeyboardEvent) -> bool {
        matches!(&event.key, Key::Named(NamedKey::Delete))
    }
}
