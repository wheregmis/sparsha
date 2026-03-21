//! List widget for dynamic collections of child widgets.

use crate::{AccessibilityInfo, AccessibilityRole, EventContext, PaintContext, Widget};
use sparsh_layout::WidgetId;
use taffy::prelude::*;

/// Layout direction for list items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListDirection {
    /// Stack items vertically.
    #[default]
    Vertical,
    /// Stack items horizontally.
    Horizontal,
}

/// A dynamic list container that owns item widgets.
pub struct List {
    id: WidgetId,
    items: Vec<Box<dyn Widget>>,
    style: Style,
}

impl List {
    /// Create a new empty vertical list.
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            items: Vec::new(),
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        }
    }

    /// Set the list direction.
    pub fn direction(mut self, direction: ListDirection) -> Self {
        self.style.flex_direction = match direction {
            ListDirection::Vertical => FlexDirection::Column,
            ListDirection::Horizontal => FlexDirection::Row,
        };
        self
    }

    /// Make list vertical.
    pub fn vertical(self) -> Self {
        self.direction(ListDirection::Vertical)
    }

    /// Make list horizontal.
    pub fn horizontal(self) -> Self {
        self.direction(ListDirection::Horizontal)
    }

    /// Set item gap.
    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = Size {
            width: length(gap),
            height: length(gap),
        };
        self
    }

    /// Set padding on all sides.
    pub fn padding(mut self, all: f32) -> Self {
        self.style.padding = Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        self
    }

    /// Fill available width.
    pub fn fill_width(mut self) -> Self {
        self.style.size.width = percent(1.0);
        self
    }

    /// Fill available height.
    pub fn fill_height(mut self) -> Self {
        self.style.size.height = percent(1.0);
        self
    }

    /// Fill width and height.
    pub fn fill(mut self) -> Self {
        self.style.size = Size {
            width: percent(1.0),
            height: percent(1.0),
        };
        self
    }

    /// Set list items.
    pub fn with_items(mut self, items: Vec<Box<dyn Widget>>) -> Self {
        self.items = items;
        self
    }

    /// Replace all items at runtime.
    pub fn set_items(&mut self, items: Vec<Box<dyn Widget>>) {
        self.items = items;
    }

    /// Append an item widget.
    pub fn push_item(&mut self, widget: impl Widget + 'static) {
        self.items.push(Box::new(widget));
    }

    /// Append a boxed item widget.
    pub fn push_boxed_item(&mut self, widget: Box<dyn Widget>) {
        self.items.push(widget);
    }

    /// Remove the item at index.
    pub fn remove_item(&mut self, index: usize) -> Option<Box<dyn Widget>> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// Remove all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if list has no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for List {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn event(&mut self, _ctx: &mut EventContext, _event: &sparsh_input::InputEvent) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.items
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.items
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        Some(AccessibilityInfo::new(AccessibilityRole::List))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn list_runtime_item_operations_work() {
        let mut list = List::new().vertical();
        assert!(list.is_empty());

        list.push_item(Text::new("A"));
        list.push_item(Text::new("B"));
        assert_eq!(list.len(), 2);

        let removed = list.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(list.len(), 1);

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn list_set_items_replaces_children() {
        let mut list = List::new();
        list.set_items(vec![Box::new(Text::new("One")), Box::new(Text::new("Two"))]);
        assert_eq!(list.children().len(), 2);

        list.set_items(vec![Box::new(Text::new("Only"))]);
        assert_eq!(list.children_mut().len(), 1);
    }
}
