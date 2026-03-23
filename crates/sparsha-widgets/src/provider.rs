//! Subtree-scoped typed provider widget.

use crate::{BuildContext, IntoWidget, PaintContext, Widget};
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;

/// A typed subtree-scoped provider for rebuild-time context values.
pub struct Provider<T: Clone + 'static> {
    id: WidgetId,
    value: T,
    child: Box<dyn Widget>,
}

impl<T: Clone + 'static> Provider<T> {
    /// Create a new provider wrapping a single child subtree.
    pub fn new(value: T, child: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            value,
            child: child.into_widget(),
        }
    }
}

impl<T: Clone + 'static> Widget for Provider<T> {
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

    fn event(&mut self, _ctx: &mut crate::EventContext, _event: &InputEvent) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn enter_build_scope(&self, ctx: &mut BuildContext) {
        ctx.push_context(self.value.clone());
    }

    fn exit_build_scope(&self, ctx: &mut BuildContext) {
        ctx.pop_context::<T>();
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        self.child.measure(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::Provider;
    use crate::{BuildContext, IntoWidget, Text, ViewportInfo, Widget};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct ContextProbe<T: Clone + 'static> {
        seen: Rc<RefCell<Option<T>>>,
        viewport: Rc<RefCell<Option<ViewportInfo>>>,
    }

    impl<T: Clone + 'static> ContextProbe<T> {
        fn new(seen: Rc<RefCell<Option<T>>>, viewport: Rc<RefCell<Option<ViewportInfo>>>) -> Self {
            Self { seen, viewport }
        }
    }

    impl<T: Clone + 'static> Widget for ContextProbe<T> {
        fn id(&self) -> sparsha_layout::WidgetId {
            sparsha_layout::WidgetId::default()
        }

        fn set_id(&mut self, _id: sparsha_layout::WidgetId) {}

        fn paint(&self, _ctx: &mut crate::PaintContext) {}

        fn rebuild(&mut self, ctx: &mut BuildContext) {
            *self.seen.borrow_mut() = ctx.resource::<T>();
            *self.viewport.borrow_mut() = ctx.resource::<ViewportInfo>();
        }
    }

    fn rebuild_widget(widget: &mut dyn Widget, ctx: &mut BuildContext, path: &mut Vec<usize>) {
        ctx.set_path(path);
        widget.rebuild(ctx);
        widget.enter_build_scope(ctx);
        let child_keys: Vec<_> = (0..widget.children().len())
            .map(|index| widget.child_path_key(index))
            .collect();
        for (index, child) in widget.children_mut().iter_mut().enumerate() {
            path.push(child_keys[index]);
            rebuild_widget(child.as_mut(), ctx, path);
            path.pop();
        }
        widget.exit_build_scope(ctx);
    }

    #[test]
    fn custom_widget_reads_provided_value_during_rebuild() {
        let seen = Rc::new(RefCell::new(None::<String>));
        let viewport = Rc::new(RefCell::new(None::<ViewportInfo>));
        let mut root = Provider::new(
            String::from("provided"),
            ContextProbe::new(seen.clone(), viewport.clone()),
        );
        let mut build = BuildContext::default();
        build.insert_resource(ViewportInfo::new(1280.0, 720.0));
        let mut path = Vec::new();

        rebuild_widget(&mut root, &mut build, &mut path);

        assert_eq!(seen.borrow().as_deref(), Some("provided"));
        assert_eq!(viewport.borrow().as_ref().map(|it| it.width), Some(1280.0));
    }

    #[test]
    fn nested_provider_uses_nearest_ancestor_value() {
        let seen = Rc::new(RefCell::new(None::<String>));
        let viewport = Rc::new(RefCell::new(None::<ViewportInfo>));
        let mut root = Provider::new(
            String::from("outer"),
            Provider::new(
                String::from("inner"),
                ContextProbe::new(seen.clone(), viewport.clone()),
            ),
        );
        let mut build = BuildContext::default();
        let mut path = Vec::new();

        rebuild_widget(&mut root, &mut build, &mut path);

        assert_eq!(seen.borrow().as_deref(), Some("inner"));
    }

    #[test]
    fn provider_is_layout_and_measure_transparent() {
        let child = Text::builder().content("provided").build();
        let provider = Provider::new(String::from("value"), child);

        assert_eq!(provider.children().len(), 1);
        assert_eq!(provider.style(), provider.children()[0].style());
    }

    #[test]
    fn provider_accepts_boxed_into_widget_children() {
        let child: Box<dyn Widget> = Text::builder().content("boxed").build().into_widget();
        let provider = Provider::new(String::from("value"), child);

        assert_eq!(provider.children().len(), 1);
    }
}
