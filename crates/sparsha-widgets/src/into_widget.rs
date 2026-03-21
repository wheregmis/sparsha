use crate::Widget;

/// Converts owned widget values into `Box<dyn Widget>` for APIs that accept widgets by value.
pub trait IntoWidget {
    /// Box this value as a widget for APIs that store owned widget trait objects.
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
    /// Return the boxed widget unchanged for the identity conversion case.
    fn into_widget(self) -> Box<dyn Widget> {
        self
    }
}
