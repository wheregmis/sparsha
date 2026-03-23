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
use std::cell::Cell;

/// Platform-specific shortcut interpretation profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutProfile {
    /// Command/Meta is treated as the primary shortcut modifier.
    CommandPrimary,
    /// Control is treated as the primary shortcut modifier.
    ControlPrimary,
}

impl ShortcutProfile {
    /// Resolve the default profile for the current build target.
    pub const fn current_target() -> Self {
        #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
        {
            Self::CommandPrimary
        }

        #[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
        {
            Self::ControlPrimary
        }
    }

    /// Return the modifier bitmask that represents the primary shortcut modifier.
    pub const fn primary_modifiers(self) -> Modifiers {
        match self {
            Self::CommandPrimary => Modifiers::META,
            Self::ControlPrimary => Modifiers::CONTROL,
        }
    }
}

thread_local! {
    static ACTIVE_SHORTCUT_PROFILE: Cell<Option<ShortcutProfile>> = const { Cell::new(None) };
}

/// Run the provided closure with an explicitly selected shortcut profile.
pub fn with_shortcut_profile<R>(profile: ShortcutProfile, f: impl FnOnce() -> R) -> R {
    ACTIVE_SHORTCUT_PROFILE.with(|slot| {
        let previous = slot.replace(Some(profile));
        let result = f();
        slot.set(previous);
        result
    })
}

/// Return the currently active shortcut profile.
pub fn active_shortcut_profile() -> ShortcutProfile {
    ACTIVE_SHORTCUT_PROFILE
        .with(|slot| slot.get())
        .unwrap_or_else(ShortcutProfile::current_target)
}

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
    /// Prefer `primary_modifier_for` in shared runtime code so the platform
    /// layer can select the profile explicitly.
    pub fn primary_modifier(modifiers: Modifiers) -> bool {
        primary_modifier_for(active_shortcut_profile(), modifiers)
    }

    /// Return whether the profile-primary shortcut modifier is active.
    pub fn primary_modifier_for(profile: ShortcutProfile, modifiers: Modifiers) -> bool {
        match profile {
            ShortcutProfile::CommandPrimary => modifiers.meta(),
            ShortcutProfile::ControlPrimary => modifiers.ctrl(),
        }
    }

    /// Check if this is the primary copy shortcut.
    pub fn is_copy(event: &KeyboardEvent) -> bool {
        is_copy_for(active_shortcut_profile(), event)
    }

    /// Check if this is the primary copy shortcut for the given profile.
    pub fn is_copy_for(profile: ShortcutProfile, event: &KeyboardEvent) -> bool {
        primary_modifier_for(profile, event.modifiers) && is_char(event, 'c')
    }

    /// Check if this is the primary paste shortcut.
    pub fn is_paste(event: &KeyboardEvent) -> bool {
        is_paste_for(active_shortcut_profile(), event)
    }

    /// Check if this is the primary paste shortcut for the given profile.
    pub fn is_paste_for(profile: ShortcutProfile, event: &KeyboardEvent) -> bool {
        primary_modifier_for(profile, event.modifiers) && is_char(event, 'v')
    }

    /// Check if this is the primary cut shortcut.
    pub fn is_cut(event: &KeyboardEvent) -> bool {
        is_cut_for(active_shortcut_profile(), event)
    }

    /// Check if this is the primary cut shortcut for the given profile.
    pub fn is_cut_for(profile: ShortcutProfile, event: &KeyboardEvent) -> bool {
        primary_modifier_for(profile, event.modifiers) && is_char(event, 'x')
    }

    /// Check if this is the primary select-all shortcut.
    pub fn is_select_all(event: &KeyboardEvent) -> bool {
        is_select_all_for(active_shortcut_profile(), event)
    }

    /// Check if this is the primary select-all shortcut for the given profile.
    pub fn is_select_all_for(profile: ShortcutProfile, event: &KeyboardEvent) -> bool {
        primary_modifier_for(profile, event.modifiers) && is_char(event, 'a')
    }

    /// Check if this is the primary undo shortcut.
    pub fn is_undo(event: &KeyboardEvent) -> bool {
        is_undo_for(active_shortcut_profile(), event)
    }

    /// Check if this is the primary undo shortcut for the given profile.
    pub fn is_undo_for(profile: ShortcutProfile, event: &KeyboardEvent) -> bool {
        primary_modifier_for(profile, event.modifiers)
            && !event.modifiers.shift()
            && is_char(event, 'z')
    }

    /// Check if this is the primary redo shortcut.
    pub fn is_redo(event: &KeyboardEvent) -> bool {
        is_redo_for(active_shortcut_profile(), event)
    }

    /// Check if this is the primary redo shortcut for the given profile.
    pub fn is_redo_for(profile: ShortcutProfile, event: &KeyboardEvent) -> bool {
        (primary_modifier_for(profile, event.modifiers)
            && event.modifiers.shift()
            && is_char(event, 'z'))
            || (primary_modifier_for(profile, event.modifiers) && is_char(event, 'y'))
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
