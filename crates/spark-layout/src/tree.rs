//! Layout tree that wraps taffy for flexbox layout computation.

use slotmap::{new_key_type, SlotMap};
use spark_core::Rect;
use taffy::{prelude::*, TaffyTree};

new_key_type! {
    /// Unique identifier for a widget in the layout tree.
    pub struct WidgetId;
}

/// Mapping between WidgetId and taffy NodeId.
struct NodeMapping {
    widget_to_node: SlotMap<WidgetId, NodeId>,
    node_to_widget: std::collections::HashMap<NodeId, WidgetId>,
}

impl NodeMapping {
    fn new() -> Self {
        Self {
            widget_to_node: SlotMap::with_key(),
            node_to_widget: std::collections::HashMap::new(),
        }
    }

    fn insert(&mut self, node_id: NodeId) -> WidgetId {
        let widget_id = self.widget_to_node.insert(node_id);
        self.node_to_widget.insert(node_id, widget_id);
        widget_id
    }

    fn get_node(&self, widget_id: WidgetId) -> Option<NodeId> {
        self.widget_to_node.get(widget_id).copied()
    }

    fn get_widget(&self, node_id: NodeId) -> Option<WidgetId> {
        self.node_to_widget.get(&node_id).copied()
    }

    fn remove(&mut self, widget_id: WidgetId) -> Option<NodeId> {
        if let Some(node_id) = self.widget_to_node.remove(widget_id) {
            self.node_to_widget.remove(&node_id);
            Some(node_id)
        } else {
            None
        }
    }
}

/// The layout tree manages widget layout using taffy flexbox.
pub struct LayoutTree {
    taffy: TaffyTree<()>,
    mapping: NodeMapping,
    root: Option<WidgetId>,
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutTree {
    /// Create a new empty layout tree.
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            mapping: NodeMapping::new(),
            root: None,
        }
    }

    /// Create a new node with the given style.
    pub fn new_leaf(&mut self, style: Style) -> WidgetId {
        let node_id = self.taffy.new_leaf(style).expect("create leaf node");
        self.mapping.insert(node_id)
    }

    /// Create a new node with children.
    pub fn new_with_children(&mut self, style: Style, children: &[WidgetId]) -> WidgetId {
        let child_nodes: Vec<NodeId> = children
            .iter()
            .filter_map(|id| self.mapping.get_node(*id))
            .collect();
        let node_id = self
            .taffy
            .new_with_children(style, &child_nodes)
            .expect("create node with children");
        self.mapping.insert(node_id)
    }

    /// Set the root widget.
    pub fn set_root(&mut self, widget_id: WidgetId) {
        self.root = Some(widget_id);
    }

    /// Get the root widget.
    pub fn root(&self) -> Option<WidgetId> {
        self.root
    }

    /// Update the style of a node.
    pub fn set_style(&mut self, widget_id: WidgetId, style: Style) {
        if let Some(node_id) = self.mapping.get_node(widget_id) {
            self.taffy.set_style(node_id, style).ok();
        }
    }

    /// Add a child to a parent node.
    pub fn add_child(&mut self, parent: WidgetId, child: WidgetId) {
        if let (Some(parent_node), Some(child_node)) =
            (self.mapping.get_node(parent), self.mapping.get_node(child))
        {
            self.taffy.add_child(parent_node, child_node).ok();
        }
    }

    /// Remove a child from a parent node.
    pub fn remove_child(&mut self, parent: WidgetId, child: WidgetId) {
        if let (Some(parent_node), Some(child_node)) =
            (self.mapping.get_node(parent), self.mapping.get_node(child))
        {
            self.taffy.remove_child(parent_node, child_node).ok();
        }
    }

    /// Remove a node from the tree.
    pub fn remove(&mut self, widget_id: WidgetId) {
        if let Some(node_id) = self.mapping.remove(widget_id) {
            self.taffy.remove(node_id).ok();
        }
    }

    /// Compute the layout for the given available space.
    pub fn compute_layout(&mut self, available_width: f32, available_height: f32) {
        if let Some(root_id) = self.root {
            if let Some(node_id) = self.mapping.get_node(root_id) {
                self.taffy
                    .compute_layout(
                        node_id,
                        Size {
                            width: AvailableSpace::Definite(available_width),
                            height: AvailableSpace::Definite(available_height),
                        },
                    )
                    .ok();
            }
        }
    }

    /// Get the computed layout for a widget.
    pub fn get_layout(&self, widget_id: WidgetId) -> Option<ComputedLayout> {
        let node_id = self.mapping.get_node(widget_id)?;
        let layout = self.taffy.layout(node_id).ok()?;
        Some(ComputedLayout {
            bounds: Rect::new(
                layout.location.x,
                layout.location.y,
                layout.size.width,
                layout.size.height,
            ),
        })
    }

    /// Get the computed layout for a widget with absolute position (accumulated from ancestors).
    pub fn get_absolute_layout(&self, widget_id: WidgetId) -> Option<ComputedLayout> {
        let node_id = self.mapping.get_node(widget_id)?;

        // Accumulate positions from ancestors
        let mut x = 0.0;
        let mut y = 0.0;

        // Walk up to root to accumulate positions
        let mut current = Some(node_id);
        while let Some(node) = current {
            if let Ok(layout) = self.taffy.layout(node) {
                x += layout.location.x;
                y += layout.location.y;
            }
            current = self.taffy.parent(node);
        }

        // Get this node's size
        let layout = self.taffy.layout(node_id).ok()?;

        Some(ComputedLayout {
            bounds: Rect::new(x, y, layout.size.width, layout.size.height),
        })
    }

    /// Traverse the tree depth-first, calling the callback for each widget.
    pub fn traverse<F>(&self, mut callback: F)
    where
        F: FnMut(WidgetId, &ComputedLayout, usize),
    {
        if let Some(root_id) = self.root {
            self.traverse_node(root_id, 0.0, 0.0, 0, &mut callback);
        }
    }

    fn traverse_node<F>(
        &self,
        widget_id: WidgetId,
        parent_x: f32,
        parent_y: f32,
        depth: usize,
        callback: &mut F,
    ) where
        F: FnMut(WidgetId, &ComputedLayout, usize),
    {
        if let Some(node_id) = self.mapping.get_node(widget_id) {
            if let Ok(layout) = self.taffy.layout(node_id) {
                let absolute_x = parent_x + layout.location.x;
                let absolute_y = parent_y + layout.location.y;

                let computed = ComputedLayout {
                    bounds: Rect::new(
                        absolute_x,
                        absolute_y,
                        layout.size.width,
                        layout.size.height,
                    ),
                };

                callback(widget_id, &computed, depth);

                // Traverse children
                if let Ok(children) = self.taffy.children(node_id) {
                    for child_node in children {
                        if let Some(child_widget) = self.mapping.get_widget(child_node) {
                            self.traverse_node(
                                child_widget,
                                absolute_x,
                                absolute_y,
                                depth + 1,
                                callback,
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Computed layout result for a widget.
#[derive(Clone, Copy, Debug)]
pub struct ComputedLayout {
    /// The absolute bounds of the widget in pixels.
    pub bounds: Rect,
}

impl ComputedLayout {
    /// Create a new computed layout.
    pub fn new(bounds: Rect) -> Self {
        Self { bounds }
    }
}

/// Helper to create common styles.
pub mod styles {
    use taffy::prelude::*;

    /// Create a flex container style (column direction).
    pub fn flex_column() -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        }
    }

    /// Create a flex container style (row direction).
    pub fn flex_row() -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            ..Default::default()
        }
    }

    /// Create a style with fixed size.
    pub fn fixed(width: f32, height: f32) -> Style {
        Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            ..Default::default()
        }
    }

    /// Create a style that fills available space.
    pub fn fill() -> Style {
        Style {
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        }
    }

    /// Create a style with padding.
    pub fn with_padding(mut style: Style, all: f32) -> Style {
        style.padding = Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        style
    }

    /// Create a style with margin.
    pub fn with_margin(mut style: Style, all: f32) -> Style {
        style.margin = Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        style
    }

    /// Create a style with gap between children.
    pub fn with_gap(mut style: Style, gap: f32) -> Style {
        style.gap = Size {
            width: length(gap),
            height: length(gap),
        };
        style
    }

    /// Center children both horizontally and vertically.
    pub fn centered(mut style: Style) -> Style {
        style.justify_content = Some(JustifyContent::Center);
        style.align_items = Some(AlignItems::Center);
        style
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taffy::prelude::{
        AlignItems, Dimension, Display, FlexDirection, JustifyContent, LengthPercentage,
        LengthPercentageAuto,
    };

    #[test]
    fn layout_tree_new_root_none() {
        let tree = LayoutTree::new();
        assert!(tree.root().is_none());
    }

    #[test]
    fn layout_tree_one_leaf_get_layout() {
        let mut tree = LayoutTree::new();
        let leaf = tree.new_leaf(styles::fixed(100.0, 50.0));
        tree.set_root(leaf);
        tree.compute_layout(200.0, 200.0);
        let layout = tree.get_layout(leaf).unwrap();
        assert_eq!(layout.bounds.x, 0.0);
        assert_eq!(layout.bounds.y, 0.0);
        assert_eq!(layout.bounds.width, 100.0);
        assert_eq!(layout.bounds.height, 50.0);
        let abs = tree.get_absolute_layout(leaf).unwrap();
        assert_eq!(abs.bounds.x, 0.0);
        assert_eq!(abs.bounds.y, 0.0);
        assert_eq!(abs.bounds.width, 100.0);
        assert_eq!(abs.bounds.height, 50.0);
    }

    #[test]
    fn layout_tree_two_nodes_traverse() {
        let mut tree = LayoutTree::new();
        let child = tree.new_leaf(styles::fixed(50.0, 25.0));
        let root_style = styles::with_padding(styles::flex_column(), 10.0);
        let root = tree.new_with_children(root_style, &[child]);
        tree.set_root(root);
        tree.compute_layout(200.0, 200.0);
        let child_layout = tree.get_layout(child).unwrap();
        assert_eq!(child_layout.bounds.width, 50.0);
        assert_eq!(child_layout.bounds.height, 25.0);
        let mut count = 0;
        tree.traverse(|_id, _layout, _depth| count += 1);
        assert_eq!(count, 2);
    }

    #[test]
    fn styles_flex_column_row() {
        let col = styles::flex_column();
        assert_eq!(col.display, Display::Flex);
        assert_eq!(col.flex_direction, FlexDirection::Column);
        let row = styles::flex_row();
        assert_eq!(row.display, Display::Flex);
        assert_eq!(row.flex_direction, FlexDirection::Row);
    }

    #[test]
    fn styles_fixed_fill() {
        let s = styles::fixed(120.0, 60.0);
        assert_eq!(s.size.width, Dimension::length(120.0));
        assert_eq!(s.size.height, Dimension::length(60.0));
        let f = styles::fill();
        assert_eq!(f.size.width, Dimension::percent(1.0));
        assert_eq!(f.size.height, Dimension::percent(1.0));
    }

    #[test]
    fn styles_with_padding_margin_gap() {
        let base = styles::flex_column();
        let s = styles::with_padding(base, 10.0);
        assert_eq!(s.padding.left, LengthPercentage::length(10.0));
        assert_eq!(s.padding.right, LengthPercentage::length(10.0));
        let s2 = styles::with_margin(styles::flex_row(), 5.0);
        assert_eq!(s2.margin.left, LengthPercentageAuto::length(5.0));
        let s3 = styles::with_gap(styles::flex_column(), 8.0);
        assert_eq!(s3.gap.width, LengthPercentage::length(8.0));
        assert_eq!(s3.gap.height, LengthPercentage::length(8.0));
    }

    #[test]
    fn styles_centered() {
        let s = styles::centered(styles::flex_column());
        assert_eq!(s.justify_content, Some(JustifyContent::Center));
        assert_eq!(s.align_items, Some(AlignItems::Center));
    }
}
