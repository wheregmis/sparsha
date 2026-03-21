//! Focus management for widgets.

use sparsha_layout::WidgetId;

/// Manages keyboard focus for widgets.
#[derive(Debug, Default)]
pub struct FocusManager {
    /// The currently focused widget.
    focused: Option<WidgetId>,
    /// Stack of widgets that can receive focus (in tab order).
    focusable: Vec<WidgetId>,
}

impl FocusManager {
    /// Create a new focus manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently focused widget.
    pub fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    /// Check if a widget has focus.
    pub fn has_focus(&self, widget_id: WidgetId) -> bool {
        self.focused == Some(widget_id)
    }

    /// Set focus to a specific widget.
    pub fn set_focus(&mut self, widget_id: WidgetId) {
        self.focused = Some(widget_id);
    }

    /// Clear focus (no widget is focused).
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// Register a widget as focusable (in tab order).
    pub fn register_focusable(&mut self, widget_id: WidgetId) {
        if !self.focusable.contains(&widget_id) {
            self.focusable.push(widget_id);
        }
    }

    /// Unregister a focusable widget.
    pub fn unregister_focusable(&mut self, widget_id: WidgetId) {
        self.focusable.retain(|id| *id != widget_id);
        if self.focused == Some(widget_id) {
            self.focused = None;
        }
    }

    /// Clear all focusable widgets (call at start of each frame).
    pub fn clear_focusable(&mut self) {
        self.focusable.clear();
    }

    /// Move focus to the next focusable widget (Tab).
    pub fn focus_next(&mut self) {
        if self.focusable.is_empty() {
            return;
        }

        let next_idx = match self.focused {
            Some(current) => self
                .focusable
                .iter()
                .position(|id| *id == current)
                .map(|idx| (idx + 1) % self.focusable.len())
                .unwrap_or(0),
            None => 0,
        };

        self.focused = self.focusable.get(next_idx).copied();
    }

    /// Move focus to the previous focusable widget (Shift+Tab).
    pub fn focus_previous(&mut self) {
        if self.focusable.is_empty() {
            return;
        }

        let prev_idx = match self.focused {
            Some(current) => self
                .focusable
                .iter()
                .position(|id| *id == current)
                .map(|idx| {
                    if idx == 0 {
                        self.focusable.len() - 1
                    } else {
                        idx - 1
                    }
                })
                .unwrap_or(self.focusable.len() - 1),
            None => self.focusable.len() - 1,
        };

        self.focused = self.focusable.get(prev_idx).copied();
    }

    /// Get the number of focusable widgets.
    pub fn focusable_count(&self) -> usize {
        self.focusable.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsha_layout::{styles, LayoutTree};

    fn two_widget_ids() -> (WidgetId, WidgetId) {
        let mut tree = LayoutTree::new();
        let a = tree.new_leaf(styles::fixed(10.0, 10.0));
        let b = tree.new_leaf(styles::fixed(10.0, 10.0));
        (a, b)
    }

    #[test]
    fn focus_manager_new() {
        let fm = FocusManager::new();
        assert!(fm.focused().is_none());
        assert_eq!(fm.focusable_count(), 0);
    }

    #[test]
    fn focus_set_and_clear() {
        let (a, _) = two_widget_ids();
        let mut fm = FocusManager::new();
        fm.set_focus(a);
        assert_eq!(fm.focused(), Some(a));
        assert!(fm.has_focus(a));
        fm.clear_focus();
        assert!(fm.focused().is_none());
    }

    #[test]
    fn focus_next_wraps() {
        let (a, b) = two_widget_ids();
        let mut fm = FocusManager::new();
        fm.register_focusable(a);
        fm.register_focusable(b);
        assert_eq!(fm.focusable_count(), 2);
        fm.focus_next();
        assert_eq!(fm.focused(), Some(a));
        fm.focus_next();
        assert_eq!(fm.focused(), Some(b));
        fm.focus_next();
        assert_eq!(fm.focused(), Some(a));
    }

    #[test]
    fn focus_previous_wraps() {
        let (a, b) = two_widget_ids();
        let mut fm = FocusManager::new();
        fm.register_focusable(a);
        fm.register_focusable(b);
        fm.focus_previous();
        assert_eq!(fm.focused(), Some(b));
        fm.focus_previous();
        assert_eq!(fm.focused(), Some(a));
    }

    #[test]
    fn unregister_clears_focus() {
        let (a, b) = two_widget_ids();
        let mut fm = FocusManager::new();
        fm.register_focusable(a);
        fm.register_focusable(b);
        fm.set_focus(a);
        fm.unregister_focusable(a);
        assert!(fm.focused().is_none());
        assert_eq!(fm.focusable_count(), 1);
    }
}
