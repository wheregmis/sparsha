use crate::Widget;

/// Conversion into a boxed widget for APIs that should accept normal widget values.
pub trait IntoWidget {
    fn into_widget(self) -> Box<dyn Widget>;
}

impl<T> IntoWidget for T
where
    T: Widget + 'static,
{
    fn into_widget(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}

impl IntoWidget for Box<dyn Widget> {
    fn into_widget(self) -> Box<dyn Widget> {
        self
    }
}
