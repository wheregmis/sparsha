//! Accessibility override wrapper for a single child subtree.

use crate::{
    accessibility::{AccessibilityInfo, AccessibilityRole},
    EventContext, PaintContext, Widget,
};
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;

/// Single-child accessibility override wrapper.
pub struct Semantics {
    id: WidgetId,
    child: Box<dyn Widget>,
    info: AccessibilityInfo,
}

impl Semantics {
    pub fn new(child: impl Widget + 'static) -> Self {
        Self {
            id: WidgetId::default(),
            child: Box::new(child),
            info: AccessibilityInfo::default(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.info.label = Some(label.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.info.value = Some(value.into());
        self
    }

    pub fn role(mut self, role: AccessibilityRole) -> Self {
        self.info.role = Some(role);
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.info.hidden = hidden;
        self
    }
}

impl Widget for Semantics {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> taffy::Style {
        self.child.style()
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        self.info.has_metadata().then_some(self.info.clone())
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        self.child.measure(ctx)
    }

    #[doc(hidden)]
    fn accessibility_merge_descendant(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn metadata_methods_populate_accessibility_info() {
        let semantics = Semantics::new(Text::builder().content("label").build())
            .label("Button label")
            .description("Helpful context")
            .value("42")
            .role(AccessibilityRole::Button)
            .hidden(true);

        assert_eq!(semantics.info.label.as_deref(), Some("Button label"));
        assert_eq!(
            semantics.info.description.as_deref(),
            Some("Helpful context")
        );
        assert_eq!(semantics.info.value.as_deref(), Some("42"));
        assert_eq!(semantics.info.role, Some(AccessibilityRole::Button));
        assert!(semantics.info.hidden);
    }
}
