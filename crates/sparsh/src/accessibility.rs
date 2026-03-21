//! Shared runtime accessibility snapshot and AccessKit conversion.

use sparsh_core::Rect;
use sparsh_widgets::{AccessibilityAction, AccessibilityInfo, AccessibilityRole};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use accesskit::{
    Action, ActionData, ActionRequest, Node, NodeId, Rect as AccessRect, Role, Toggled, Tree,
    TreeId, TreeUpdate,
};

pub(crate) const ACCESSIBILITY_ROOT_ID: u64 = 0;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AccessibilityNodeSnapshot {
    pub id: u64,
    pub path: Vec<usize>,
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub hidden: bool,
    pub disabled: bool,
    pub checked: Option<bool>,
    pub actions: Vec<AccessibilityAction>,
    pub bounds: Rect,
    pub children: Vec<u64>,
}

impl AccessibilityNodeSnapshot {
    pub(crate) fn apply_overrides(&mut self, info: AccessibilityInfo) {
        if let Some(role) = info.role {
            self.role = role;
        }
        if let Some(label) = info.label {
            self.label = Some(label);
        }
        if let Some(description) = info.description {
            self.description = Some(description);
        }
        if let Some(value) = info.value {
            self.value = Some(value);
        }
        if let Some(checked) = info.checked {
            self.checked = Some(checked);
        }
        self.hidden |= info.hidden;
        self.disabled |= info.disabled;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct AccessibilityTreeSnapshot {
    pub nodes: Vec<AccessibilityNodeSnapshot>,
    pub root_children: Vec<u64>,
    pub focus: u64,
    pub node_paths: HashMap<u64, Vec<usize>>,
}

impl AccessibilityTreeSnapshot {
    pub(crate) fn path_for_node(&self, node_id: u64) -> Option<&[usize]> {
        self.node_paths.get(&node_id).map(Vec::as_slice)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn to_tree_update(&self, window_title: &str) -> TreeUpdate {
        let mut root = Node::new(Role::Window);
        root.set_label(window_title.to_owned());
        root.set_children(
            self.root_children
                .iter()
                .copied()
                .map(NodeId)
                .collect::<Vec<_>>(),
        );

        let mut nodes = Vec::with_capacity(self.nodes.len() + 1);
        nodes.push((NodeId(ACCESSIBILITY_ROOT_ID), root));
        for node in &self.nodes {
            nodes.push((NodeId(node.id), build_accesskit_node(node)));
        }

        TreeUpdate {
            nodes,
            tree: Some(Tree::new(NodeId(ACCESSIBILITY_ROOT_ID))),
            tree_id: TreeId::ROOT,
            focus: NodeId(if self.focus == 0 {
                ACCESSIBILITY_ROOT_ID
            } else {
                self.focus
            }),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RoutedAccessibilityAction {
    pub node_id: u64,
    pub action: AccessibilityAction,
    pub value: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn action_from_accesskit(request: ActionRequest) -> Option<RoutedAccessibilityAction> {
    let action = match request.action {
        Action::Click => AccessibilityAction::Click,
        Action::Focus => AccessibilityAction::Focus,
        Action::SetValue => AccessibilityAction::SetValue,
        Action::ScrollUp => AccessibilityAction::ScrollUp,
        Action::ScrollDown => AccessibilityAction::ScrollDown,
        Action::ScrollLeft => AccessibilityAction::ScrollLeft,
        Action::ScrollRight => AccessibilityAction::ScrollRight,
        _ => return None,
    };

    let value = match request.data {
        Some(ActionData::Value(value)) => Some(value.into_string()),
        _ => None,
    };

    Some(RoutedAccessibilityAction {
        node_id: request.target_node.0,
        action,
        value,
    })
}

pub(crate) fn accessibility_node_id(path: &[usize]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x00000100000001B3;

    let mut hash = OFFSET_BASIS;
    hash ^= 0xFF;
    hash = hash.wrapping_mul(PRIME);
    for index in path {
        for byte in index.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(PRIME);
        }
        hash ^= 0xFE;
        hash = hash.wrapping_mul(PRIME);
    }
    if hash == ACCESSIBILITY_ROOT_ID {
        1
    } else {
        hash
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_accesskit_node(snapshot: &AccessibilityNodeSnapshot) -> Node {
    let mut node = Node::new(accesskit_role(snapshot.role));
    node.set_children(
        snapshot
            .children
            .iter()
            .copied()
            .map(NodeId)
            .collect::<Vec<_>>(),
    );
    node.set_bounds(accesskit_rect(snapshot.bounds));

    if let Some(label) = &snapshot.label {
        node.set_label(label.clone());
    }
    if let Some(description) = &snapshot.description {
        node.set_description(description.clone());
    }
    if let Some(value) = &snapshot.value {
        node.set_value(value.clone());
    }
    if snapshot.hidden {
        node.set_hidden();
    }
    if snapshot.disabled {
        node.set_disabled();
    }
    if let Some(checked) = snapshot.checked {
        node.set_toggled(Toggled::from(checked));
    }
    for action in &snapshot.actions {
        node.add_action(accesskit_action(*action));
    }

    node
}

#[cfg(not(target_arch = "wasm32"))]
fn accesskit_role(value: AccessibilityRole) -> Role {
    match value {
        AccessibilityRole::GenericContainer => Role::GenericContainer,
        AccessibilityRole::Button => Role::Button,
        AccessibilityRole::CheckBox => Role::CheckBox,
        AccessibilityRole::Label => Role::Label,
        AccessibilityRole::TextInput => Role::TextInput,
        AccessibilityRole::MultilineTextInput => Role::MultilineTextInput,
        AccessibilityRole::List => Role::List,
        AccessibilityRole::ScrollView => Role::ScrollView,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn accesskit_action(value: AccessibilityAction) -> Action {
    match value {
        AccessibilityAction::Click => Action::Click,
        AccessibilityAction::Focus => Action::Focus,
        AccessibilityAction::SetValue => Action::SetValue,
        AccessibilityAction::ScrollUp => Action::ScrollUp,
        AccessibilityAction::ScrollDown => Action::ScrollDown,
        AccessibilityAction::ScrollLeft => Action::ScrollLeft,
        AccessibilityAction::ScrollRight => Action::ScrollRight,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn accesskit_rect(value: Rect) -> AccessRect {
    AccessRect {
        x0: value.x as f64,
        y0: value.y as f64,
        x1: (value.x + value.width) as f64,
        y1: (value.y + value.height) as f64,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn node_ids_are_stable_for_widget_paths() {
        assert_eq!(
            accessibility_node_id(&[0, 1, 2]),
            accessibility_node_id(&[0, 1, 2])
        );
        assert_ne!(
            accessibility_node_id(&[0, 1, 2]),
            accessibility_node_id(&[0, 2, 1])
        );
    }

    #[test]
    fn snapshot_converts_to_accesskit_tree() {
        let snapshot = AccessibilityTreeSnapshot {
            nodes: vec![AccessibilityNodeSnapshot {
                id: 42,
                path: vec![0],
                role: AccessibilityRole::Button,
                label: Some("Press".to_owned()),
                description: None,
                value: None,
                hidden: false,
                disabled: false,
                checked: None,
                actions: vec![AccessibilityAction::Click, AccessibilityAction::Focus],
                bounds: Rect::new(10.0, 20.0, 30.0, 40.0),
                children: Vec::new(),
            }],
            root_children: vec![42],
            focus: 42,
            node_paths: HashMap::from([(42, vec![0])]),
        };

        let update = snapshot.to_tree_update("Test");
        assert_eq!(update.focus, NodeId(42));
        assert_eq!(update.nodes.len(), 2);
    }
}
