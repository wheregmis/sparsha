//! Widget trait and response types.

use crate::{
    accessibility::{AccessibilityAction, AccessibilityInfo},
    text_editor::TextEditorState,
};
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;

/// Runtime traversal mode for a widget child subtree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetChildMode {
    Active,
    PaintOnly,
}

/// The core widget trait that all UI components implement.
pub trait Widget {
    /// Get the widget's unique ID.
    fn id(&self) -> WidgetId;

    /// Set the widget's ID (called by the framework during tree construction).
    fn set_id(&mut self, id: WidgetId);

    /// Get the layout style for this widget.
    fn style(&self) -> taffy::Style {
        taffy::Style::default()
    }

    /// Paint this widget to the draw list.
    fn paint(&self, ctx: &mut super::PaintContext);

    /// Called after children have been painted.
    /// Use this to clean up clips/transforms pushed in paint().
    fn paint_after_children(&self, _ctx: &mut super::PaintContext) {
        // Default: no-op
    }

    /// Optional draw-heavy surface hook for runtimes that support hybrid rendering.
    ///
    /// Returning `Some(&DrawSurface)` does not replace the widget's normal paint path.
    /// Runtimes should render the surface scene and still allow the widget to paint retained
    /// overlays and descendants through `paint()` / `paint_after_children()`.
    fn draw_surface(&self) -> Option<&crate::DrawSurface> {
        None
    }

    /// Rebuild dynamic children before layout.
    fn rebuild(&mut self, _ctx: &mut super::BuildContext) {
        // Default: no-op
    }

    /// Persist live runtime state before a rebuild may replace this widget.
    fn persist_build_state(&self, _ctx: &mut super::BuildContext) {
        // Default: no-op
    }

    /// Enter a rebuild-time scoped resource boundary for descendants.
    #[doc(hidden)]
    fn enter_build_scope(&self, _ctx: &mut super::BuildContext) {
        // Default: no-op
    }

    /// Exit a rebuild-time scoped resource boundary for descendants.
    #[doc(hidden)]
    fn exit_build_scope(&self, _ctx: &mut super::BuildContext) {
        // Default: no-op
    }

    /// Handle an input event.
    fn event(&mut self, ctx: &mut super::EventContext, event: &InputEvent) {
        let _ = (ctx, event);
    }

    /// Get child widgets (for containers).
    fn children(&self) -> &[Box<dyn Widget>] {
        &[]
    }

    /// Get mutable child widgets.
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut []
    }

    /// Select how the runtime should treat a child subtree.
    ///
    /// `PaintOnly` children still rebuild, lay out, and paint, but they do not participate in
    /// event dispatch, focus traversal, text-editor registration, or accessibility collection.
    fn child_mode(&self, _child_position: usize) -> WidgetChildMode {
        WidgetChildMode::Active
    }

    /// Called when the widget receives focus.
    fn on_focus(&mut self) {}

    /// Called when the widget loses focus.
    fn on_blur(&mut self) {}

    /// Return a runtime-facing snapshot when the widget is a text editor.
    fn text_editor_state(&self) -> Option<TextEditorState> {
        None
    }

    /// Return accessibility metadata for this widget.
    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        None
    }

    /// Handle an accessibility action routed by the runtime.
    fn handle_accessibility_action(
        &mut self,
        _action: AccessibilityAction,
        _value: Option<String>,
    ) -> bool {
        false
    }

    /// Whether this widget can receive keyboard focus.
    fn focusable(&self) -> bool {
        false
    }

    /// Whether this widget is a scroll container.
    fn is_scroll_container(&self) -> bool {
        false
    }

    /// Logical offset applied to descendant event hit-testing.
    ///
    /// Widgets that visually translate their children without changing layout positions should
    /// return the inverse of that paint translation so pointer hit-testing stays aligned.
    fn child_event_offset(&self) -> glam::Vec2 {
        glam::Vec2::ZERO
    }

    /// Stable logical key for a child position.
    #[doc(hidden)]
    fn child_path_key(&self, child_position: usize) -> usize {
        child_position
    }

    /// Realized child slot for a stable logical key.
    #[doc(hidden)]
    fn child_slot_for_path_key(&self, key: usize) -> Option<usize> {
        Some(key)
    }

    /// Whether accessibility metadata should override the first accessible descendant.
    #[doc(hidden)]
    fn accessibility_merge_descendant(&self) -> bool {
        false
    }

    /// Measure the widget's preferred size (for intrinsic sizing).
    fn measure(&self, ctx: &mut super::LayoutContext) -> Option<(f32, f32)> {
        let _ = ctx;
        None
    }
}
