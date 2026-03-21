//! Widget trait and response types.

use sparsh_input::InputEvent;
use sparsh_layout::WidgetId;

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

    /// Called when the widget receives focus.
    fn on_focus(&mut self) {}

    /// Called when the widget loses focus.
    fn on_blur(&mut self) {}

    /// Whether this widget can receive keyboard focus.
    fn focusable(&self) -> bool {
        false
    }

    /// Whether this widget is a scroll container.
    fn is_scroll_container(&self) -> bool {
        false
    }

    /// Measure the widget's preferred size (for intrinsic sizing).
    fn measure(&self, ctx: &mut super::LayoutContext) -> Option<(f32, f32)> {
        let _ = ctx;
        None
    }
}
