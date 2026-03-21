//! Accessibility support using AccessKit.
//!
//! This module provides integration with AccessKit for cross-platform
//! accessibility support (screen readers, alternative input methods, etc.)
#![allow(dead_code)]

use accesskit::{Action, Node, NodeId, Role, Tree, TreeId, TreeUpdate};
use sparsh_layout::WidgetId;
use std::collections::HashMap;

/// Maps between Sparsh WidgetIds and AccessKit NodeIds.
pub struct AccessibilityIdMap {
    widget_to_node: HashMap<WidgetId, NodeId>,
    node_to_widget: HashMap<NodeId, WidgetId>,
    next_id: u64,
}

impl Default for AccessibilityIdMap {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityIdMap {
    pub fn new() -> Self {
        Self {
            widget_to_node: HashMap::new(),
            node_to_widget: HashMap::new(),
            // Start at 1 since 0 is reserved for the root
            next_id: 1,
        }
    }

    /// Get or create a NodeId for a WidgetId.
    pub fn get_or_create(&mut self, widget_id: WidgetId) -> NodeId {
        if let Some(&node_id) = self.widget_to_node.get(&widget_id) {
            node_id
        } else {
            let node_id = NodeId(self.next_id);
            self.next_id += 1;
            self.widget_to_node.insert(widget_id, node_id);
            self.node_to_widget.insert(node_id, widget_id);
            node_id
        }
    }

    /// Get the WidgetId for a NodeId.
    pub fn get_widget(&self, node_id: NodeId) -> Option<WidgetId> {
        self.node_to_widget.get(&node_id).copied()
    }

    /// Get the NodeId for a WidgetId.
    pub fn get_node(&self, widget_id: WidgetId) -> Option<NodeId> {
        self.widget_to_node.get(&widget_id).copied()
    }

    /// Clear all mappings.
    pub fn clear(&mut self) {
        self.widget_to_node.clear();
        self.node_to_widget.clear();
        self.next_id = 1;
    }
}

/// Accessibility information that widgets can provide.
#[derive(Clone, Debug, Default)]
pub struct AccessibleInfo {
    /// The role of this element (button, text field, etc.)
    pub role: AccessibleRole,
    /// Human-readable name/label
    pub name: Option<String>,
    /// Human-readable description
    pub description: Option<String>,
    /// Current value (for sliders, text fields, etc.)
    pub value: Option<String>,
    /// Whether the element is focusable
    pub focusable: bool,
    /// Whether the element is currently focused
    pub focused: bool,
    /// Whether the element is disabled
    pub disabled: bool,
    /// Available actions
    pub actions: Vec<AccessibleAction>,
}

/// Role of an accessible element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccessibleRole {
    /// Generic container
    #[default]
    GenericContainer,
    /// Push button
    Button,
    /// Text input field
    TextField,
    /// Static text label
    Label,
    /// Checkbox
    CheckBox,
    /// Radio button
    RadioButton,
    /// Slider
    Slider,
    /// List
    List,
    /// List item
    ListItem,
    /// Window
    Window,
    /// Scroll view
    ScrollView,
    /// Image
    Image,
}

impl From<AccessibleRole> for Role {
    fn from(role: AccessibleRole) -> Self {
        match role {
            AccessibleRole::GenericContainer => Role::GenericContainer,
            AccessibleRole::Button => Role::Button,
            AccessibleRole::TextField => Role::TextInput,
            AccessibleRole::Label => Role::Label,
            AccessibleRole::CheckBox => Role::CheckBox,
            AccessibleRole::RadioButton => Role::RadioButton,
            AccessibleRole::Slider => Role::Slider,
            AccessibleRole::List => Role::List,
            AccessibleRole::ListItem => Role::ListItem,
            AccessibleRole::Window => Role::Window,
            AccessibleRole::ScrollView => Role::ScrollView,
            AccessibleRole::Image => Role::Image,
        }
    }
}

/// Actions that assistive technologies can request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibleAction {
    /// Click/activate the element
    Click,
    /// Focus the element
    Focus,
    /// Set the element's value
    SetValue,
    /// Increment (for sliders, etc.)
    Increment,
    /// Decrement (for sliders, etc.)
    Decrement,
    /// Scroll up
    ScrollUp,
    /// Scroll down
    ScrollDown,
    /// Scroll left
    ScrollLeft,
    /// Scroll right
    ScrollRight,
}

impl From<AccessibleAction> for Action {
    fn from(action: AccessibleAction) -> Self {
        match action {
            AccessibleAction::Click => Action::Click,
            AccessibleAction::Focus => Action::Focus,
            AccessibleAction::SetValue => Action::SetValue,
            AccessibleAction::Increment => Action::Increment,
            AccessibleAction::Decrement => Action::Decrement,
            AccessibleAction::ScrollUp => Action::ScrollUp,
            AccessibleAction::ScrollDown => Action::ScrollDown,
            AccessibleAction::ScrollLeft => Action::ScrollLeft,
            AccessibleAction::ScrollRight => Action::ScrollRight,
        }
    }
}

/// Builds an AccessKit Node from AccessibleInfo.
pub fn build_node(info: &AccessibleInfo) -> Node {
    let mut node = Node::new(info.role.into());

    if let Some(ref name) = info.name {
        node.set_label(name.clone());
    }

    if let Some(ref desc) = info.description {
        node.set_description(desc.clone());
    }

    if let Some(ref value) = info.value {
        node.set_value(value.clone());
    }

    // Add Focus action to indicate the node is focusable
    if info.focusable {
        node.add_action(Action::Focus);
    }

    if info.disabled {
        node.set_disabled();
    }

    // Add available actions
    for action in &info.actions {
        node.add_action((*action).into());
    }

    node
}

/// Trait for widgets to provide accessibility information.
pub trait Accessible {
    /// Get the accessibility info for this widget.
    fn accessibility_info(&self) -> AccessibleInfo {
        AccessibleInfo::default()
    }

    /// Handle an accessibility action request.
    fn handle_accessibility_action(&mut self, _action: AccessibleAction) -> bool {
        false
    }
}

/// Manages the accessibility tree for the application.
pub struct AccessibilityManager {
    id_map: AccessibilityIdMap,
    root_id: NodeId,
}

impl Default for AccessibilityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityManager {
    pub fn new() -> Self {
        Self {
            id_map: AccessibilityIdMap::new(),
            root_id: NodeId(0), // Root is always 0
        }
    }

    /// Build an initial tree update for the application.
    pub fn build_initial_tree(&mut self, app_name: &str) -> TreeUpdate {
        // Create root window node
        let mut root_node = Node::new(Role::Window);
        root_node.set_label(app_name.to_string());

        TreeUpdate {
            nodes: vec![(self.root_id, root_node)],
            tree: Some(Tree::new(self.root_id)),
            tree_id: TreeId::ROOT,
            focus: self.root_id,
        }
    }

    /// Get or create a NodeId for a widget.
    pub fn get_node_id(&mut self, widget_id: WidgetId) -> NodeId {
        self.id_map.get_or_create(widget_id)
    }

    /// Get the WidgetId for a NodeId.
    pub fn get_widget_id(&self, node_id: NodeId) -> Option<WidgetId> {
        self.id_map.get_widget(node_id)
    }

    /// Get the root NodeId.
    pub fn root_id(&self) -> NodeId {
        self.root_id
    }

    /// Clear all mappings (call when rebuilding the tree).
    pub fn clear(&mut self) {
        self.id_map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_mapping() {
        let mut manager = AccessibilityManager::new();
        let widget_id = WidgetId::default();

        let node_id1 = manager.get_node_id(widget_id);
        let node_id2 = manager.get_node_id(widget_id);

        // Same widget should get same node ID
        assert_eq!(node_id1, node_id2);

        // Should be able to look up widget from node
        assert_eq!(manager.get_widget_id(node_id1), Some(widget_id));
    }

    #[test]
    fn test_build_node() {
        let info = AccessibleInfo {
            role: AccessibleRole::Button,
            name: Some("Click Me".to_string()),
            focusable: true,
            actions: vec![AccessibleAction::Click],
            ..Default::default()
        };

        let node = build_node(&info);
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.label(), Some("Click Me"));
    }
}
