//! Action system for semantic UI actions.
//!
//! Actions decouple what happened (an event) from what to do (the action).
//! This allows multiple input methods (keyboard, mouse, touch) to trigger
//! the same logical action.

use crate::{
    active_shortcut_profile, shortcuts, InputEvent, Key, KeyboardEvent, NamedKey, PointerButton,
    ShortcutProfile,
};

/// Built-in UI actions that have standard semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StandardAction {
    // Navigation
    /// Move focus to next focusable element
    FocusNext,
    /// Move focus to previous focusable element
    FocusPrevious,
    /// Activate/click the focused element
    Activate,
    /// Cancel current operation or close dialog
    Cancel,

    // Text editing
    /// Copy selection to clipboard
    Copy,
    /// Cut selection to clipboard
    Cut,
    /// Paste from clipboard
    Paste,
    /// Select all content
    SelectAll,
    /// Undo last action
    Undo,
    /// Redo last undone action
    Redo,
    /// Delete character before cursor
    Backspace,
    /// Delete character after cursor
    Delete,

    // Movement
    /// Move cursor/selection left
    MoveLeft,
    /// Move cursor/selection right
    MoveRight,
    /// Move cursor/selection up
    MoveUp,
    /// Move cursor/selection down
    MoveDown,
    /// Move to start of line/content
    MoveToStart,
    /// Move to end of line/content
    MoveToEnd,
    /// Move word left
    MoveWordLeft,
    /// Move word right
    MoveWordRight,

    // Selection (same as movement but extending selection)
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectToStart,
    SelectToEnd,
    SelectWordLeft,
    SelectWordRight,

    // Form actions
    /// Submit form
    Submit,
    /// Reset form
    Reset,
}

/// A user-defined action identified by a string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomAction(pub String);

impl CustomAction {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// An action that can be triggered by input events.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// A built-in standard action
    Standard(StandardAction),
    /// A custom user-defined action
    Custom(CustomAction),
}

impl From<StandardAction> for Action {
    fn from(action: StandardAction) -> Self {
        Action::Standard(action)
    }
}

impl From<CustomAction> for Action {
    fn from(action: CustomAction) -> Self {
        Action::Custom(action)
    }
}

impl From<&str> for Action {
    fn from(name: &str) -> Self {
        Action::Custom(CustomAction::new(name))
    }
}

/// Maps input events to actions using pattern matching.
pub struct ActionMapper {
    shortcut_profile: ShortcutProfile,
}

impl Default for ActionMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionMapper {
    /// Create a new action mapper.
    pub fn new() -> Self {
        Self {
            shortcut_profile: active_shortcut_profile(),
        }
    }

    /// Create an action mapper with an explicit shortcut profile.
    pub fn with_shortcut_profile(shortcut_profile: ShortcutProfile) -> Self {
        Self { shortcut_profile }
    }

    /// Return the active shortcut profile for this mapper.
    pub fn shortcut_profile(&self) -> ShortcutProfile {
        self.shortcut_profile
    }

    /// Map a keyboard event to a standard action.
    fn map_keyboard(&self, event: &KeyboardEvent) -> Option<StandardAction> {
        use StandardAction::*;

        // Check shortcuts first (they use modifiers)
        if shortcuts::is_copy_for(self.shortcut_profile, event) {
            return Some(Copy);
        }
        if shortcuts::is_cut_for(self.shortcut_profile, event) {
            return Some(Cut);
        }
        if shortcuts::is_paste_for(self.shortcut_profile, event) {
            return Some(Paste);
        }
        if shortcuts::is_select_all_for(self.shortcut_profile, event) {
            return Some(SelectAll);
        }
        if shortcuts::is_undo_for(self.shortcut_profile, event) {
            return Some(Undo);
        }
        if shortcuts::is_redo_for(self.shortcut_profile, event) {
            return Some(Redo);
        }

        // Check navigation/editing keys.
        match &event.key {
            Key::Character(value)
                if value == " "
                    && !event.modifiers.ctrl()
                    && !event.modifiers.alt()
                    && !event.modifiers.meta() =>
            {
                Some(Activate)
            }
            Key::Named(named) => match named {
                NamedKey::Tab => {
                    if event.modifiers.shift() {
                        Some(FocusPrevious)
                    } else {
                        Some(FocusNext)
                    }
                }
                NamedKey::Enter => Some(Activate),
                NamedKey::Escape => Some(Cancel),
                NamedKey::Backspace => Some(Backspace),
                NamedKey::Delete => Some(Delete),
                NamedKey::ArrowLeft => {
                    if event.modifiers.shift()
                        && shortcuts::primary_modifier_for(self.shortcut_profile, event.modifiers)
                    {
                        Some(SelectWordLeft)
                    } else if event.modifiers.shift() {
                        Some(SelectLeft)
                    } else if shortcuts::primary_modifier_for(
                        self.shortcut_profile,
                        event.modifiers,
                    ) {
                        Some(MoveWordLeft)
                    } else {
                        Some(MoveLeft)
                    }
                }
                NamedKey::ArrowRight => {
                    if event.modifiers.shift()
                        && shortcuts::primary_modifier_for(self.shortcut_profile, event.modifiers)
                    {
                        Some(SelectWordRight)
                    } else if event.modifiers.shift() {
                        Some(SelectRight)
                    } else if shortcuts::primary_modifier_for(
                        self.shortcut_profile,
                        event.modifiers,
                    ) {
                        Some(MoveWordRight)
                    } else {
                        Some(MoveRight)
                    }
                }
                NamedKey::ArrowUp => {
                    if event.modifiers.shift() {
                        Some(SelectUp)
                    } else {
                        Some(MoveUp)
                    }
                }
                NamedKey::ArrowDown => {
                    if event.modifiers.shift() {
                        Some(SelectDown)
                    } else {
                        Some(MoveDown)
                    }
                }
                NamedKey::Home => {
                    if event.modifiers.shift() {
                        Some(SelectToStart)
                    } else {
                        Some(MoveToStart)
                    }
                }
                NamedKey::End => {
                    if event.modifiers.shift() {
                        Some(SelectToEnd)
                    } else {
                        Some(MoveToEnd)
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Try to map an input event to an action.
    pub fn map_event(&self, event: &InputEvent) -> Option<Action> {
        match event {
            InputEvent::KeyDown { event: kb_event } => {
                self.map_keyboard(kb_event).map(Action::Standard)
            }
            InputEvent::PointerDown {
                button: PointerButton::Primary,
                ..
            } => Some(Action::Standard(StandardAction::Activate)),
            _ => None,
        }
    }

    /// Check if a specific action is triggered by an event.
    pub fn is_action(&self, event: &InputEvent, action: StandardAction) -> bool {
        self.map_event(event) == Some(Action::Standard(action))
    }
}

/// Callback type for action handlers.
pub type ActionHandler<T> = Box<dyn FnMut(&Action, &mut T) + Send + Sync>;

/// Context for handling actions within widgets.
pub struct ActionContext {
    mapper: ActionMapper,
    pending_actions: Vec<Action>,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionContext {
    pub fn new() -> Self {
        Self {
            mapper: ActionMapper::new(),
            pending_actions: Vec::new(),
        }
    }

    /// Get the action mapper for customization.
    pub fn mapper(&mut self) -> &mut ActionMapper {
        &mut self.mapper
    }

    /// Process an input event and return any triggered action.
    pub fn process_event(&mut self, event: &InputEvent) -> Option<Action> {
        self.mapper.map_event(event)
    }

    /// Queue an action to be handled.
    pub fn dispatch(&mut self, action: impl Into<Action>) {
        self.pending_actions.push(action.into());
    }

    /// Take all pending actions.
    pub fn take_pending(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Check if a specific standard action matches the event.
    pub fn is_action(&self, event: &InputEvent, action: StandardAction) -> bool {
        self.mapper.is_action(event, action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShortcutProfile;
    use ui_events::keyboard::Modifiers;

    #[test]
    fn test_escape_maps_to_cancel() {
        let mapper = ActionMapper::new();
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::Escape),
                ..Default::default()
            },
        };

        assert_eq!(
            mapper.map_event(&event),
            Some(Action::Standard(StandardAction::Cancel))
        );
    }

    #[test]
    fn test_enter_maps_to_activate() {
        let mapper = ActionMapper::new();
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::Enter),
                ..Default::default()
            },
        };

        assert_eq!(
            mapper.map_event(&event),
            Some(Action::Standard(StandardAction::Activate))
        );
    }

    #[test]
    fn test_space_maps_to_activate() {
        let mapper = ActionMapper::new();
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Character(" ".to_owned()),
                ..Default::default()
            },
        };

        assert_eq!(
            mapper.map_event(&event),
            Some(Action::Standard(StandardAction::Activate))
        );
    }

    #[test]
    fn test_space_with_primary_shortcut_modifiers_does_not_activate() {
        let mapper = ActionMapper::with_shortcut_profile(ShortcutProfile::ControlPrimary);
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Character(" ".to_owned()),
                modifiers: ShortcutProfile::ControlPrimary.primary_modifiers(),
                ..Default::default()
            },
        };

        assert_eq!(mapper.map_event(&event), None);
    }

    #[test]
    fn test_space_with_alt_does_not_activate() {
        let mapper = ActionMapper::new();
        let event = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Character(" ".to_owned()),
                modifiers: Modifiers::ALT,
                ..Default::default()
            },
        };

        assert_eq!(mapper.map_event(&event), None);
    }

    #[test]
    fn test_tab_maps_to_focus_navigation() {
        let mapper = ActionMapper::new();
        let forward = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::Tab),
                ..Default::default()
            },
        };
        let backward = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::Tab),
                modifiers: Modifiers::SHIFT,
                ..Default::default()
            },
        };

        assert_eq!(
            mapper.map_event(&forward),
            Some(Action::Standard(StandardAction::FocusNext))
        );
        assert_eq!(
            mapper.map_event(&backward),
            Some(Action::Standard(StandardAction::FocusPrevious))
        );
    }

    #[test]
    fn test_primary_shortcuts_map_copy_and_word_movement() {
        let mapper = ActionMapper::with_shortcut_profile(ShortcutProfile::ControlPrimary);
        let copy = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Character("c".into()),
                modifiers: ShortcutProfile::ControlPrimary.primary_modifiers(),
                ..Default::default()
            },
        };
        let move_word = InputEvent::KeyDown {
            event: KeyboardEvent {
                key: Key::Named(NamedKey::ArrowRight),
                modifiers: ShortcutProfile::ControlPrimary.primary_modifiers(),
                ..Default::default()
            },
        };

        assert_eq!(
            mapper.map_event(&copy),
            Some(Action::Standard(StandardAction::Copy))
        );
        assert_eq!(
            mapper.map_event(&move_word),
            Some(Action::Standard(StandardAction::MoveWordRight))
        );
    }
}
