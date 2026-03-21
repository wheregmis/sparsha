//! Hit testing for finding widgets under a point.

use glam::Vec2;
use sparsha_core::Rect;
use sparsha_layout::{ComputedLayout, LayoutTree, WidgetId};

/// Result of a hit test.
#[derive(Clone, Copy, Debug)]
pub struct HitTestResult {
    /// The widget that was hit.
    pub widget_id: WidgetId,
    /// The position relative to the widget's bounds.
    pub local_pos: Vec2,
    /// The depth of the widget in the tree (higher = more nested).
    pub depth: usize,
}

/// Perform a hit test to find the deepest widget at the given position.
pub fn hit_test(layout_tree: &LayoutTree, pos: Vec2) -> Option<HitTestResult> {
    let mut result: Option<HitTestResult> = None;

    layout_tree.traverse(|widget_id, computed, depth| {
        if computed.bounds.contains(pos) {
            // Prefer deeper widgets (more nested = more specific)
            if result.is_none() || depth > result.as_ref().unwrap().depth {
                result = Some(HitTestResult {
                    widget_id,
                    local_pos: Vec2::new(pos.x - computed.bounds.x, pos.y - computed.bounds.y),
                    depth,
                });
            }
        }
    });

    result
}

/// Perform a hit test with a custom filter.
pub fn hit_test_filtered<F>(layout_tree: &LayoutTree, pos: Vec2, filter: F) -> Option<HitTestResult>
where
    F: Fn(WidgetId) -> bool,
{
    let mut result: Option<HitTestResult> = None;

    layout_tree.traverse(|widget_id, computed, depth| {
        if computed.bounds.contains(pos)
            && filter(widget_id)
            && (result.is_none() || depth > result.as_ref().unwrap().depth)
        {
            result = Some(HitTestResult {
                widget_id,
                local_pos: Vec2::new(pos.x - computed.bounds.x, pos.y - computed.bounds.y),
                depth,
            });
        }
    });

    result
}

/// Check if a point is inside a rectangle.
#[allow(dead_code)]
pub fn point_in_rect(pos: Vec2, rect: &Rect) -> bool {
    rect.contains(pos)
}

/// Check if a point is inside a computed layout.
#[allow(dead_code)]
pub fn point_in_layout(pos: Vec2, layout: &ComputedLayout) -> bool {
    layout.bounds.contains(pos)
}

/// Get all widgets at a position (from front to back).
pub fn hit_test_all(layout_tree: &LayoutTree, pos: Vec2) -> Vec<HitTestResult> {
    let mut results = Vec::new();

    layout_tree.traverse(|widget_id, computed, depth| {
        if computed.bounds.contains(pos) {
            results.push(HitTestResult {
                widget_id,
                local_pos: Vec2::new(pos.x - computed.bounds.x, pos.y - computed.bounds.y),
                depth,
            });
        }
    });

    // Sort by depth descending (deepest first)
    results.sort_by_key(|b| std::cmp::Reverse(b.depth));

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use sparsha_layout::styles;

    #[test]
    fn hit_test_inside_returns_result() {
        let mut tree = LayoutTree::new();
        let leaf = tree.new_leaf(styles::fixed(100.0, 50.0));
        tree.set_root(leaf);
        tree.compute_layout(200.0, 200.0);
        let result = hit_test(&tree, Vec2::new(50.0, 25.0)).unwrap();
        assert_eq!(result.widget_id, leaf);
        assert_eq!(result.local_pos, Vec2::new(50.0, 25.0));
        assert_eq!(result.depth, 0);
    }

    #[test]
    fn hit_test_outside_returns_none() {
        let mut tree = LayoutTree::new();
        let leaf = tree.new_leaf(styles::fixed(100.0, 50.0));
        tree.set_root(leaf);
        tree.compute_layout(200.0, 200.0);
        assert!(hit_test(&tree, Vec2::new(150.0, 25.0)).is_none());
    }

    #[test]
    fn point_in_rect_and_layout() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(point_in_rect(Vec2::new(50.0, 45.0), &rect));
        assert!(!point_in_rect(Vec2::new(5.0, 5.0), &rect));
        let layout = ComputedLayout::new(rect);
        assert!(point_in_layout(Vec2::new(50.0, 45.0), &layout));
        assert!(!point_in_layout(Vec2::new(5.0, 5.0), &layout));
    }

    #[test]
    fn hit_test_all_two_overlapping() {
        let mut tree = LayoutTree::new();
        let child = tree.new_leaf(styles::fixed(80.0, 80.0));
        let root = tree.new_with_children(styles::flex_column(), &[child]);
        tree.set_root(root);
        tree.compute_layout(200.0, 200.0);
        let results = hit_test_all(&tree, Vec2::new(40.0, 40.0));
        assert_eq!(results.len(), 2);
        assert!(results[0].depth >= results[1].depth);
    }
}
