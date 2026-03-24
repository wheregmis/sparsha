//! Context types passed to widgets during layout, paint, and events.

use crate::Theme;
use sparsha_core::{Color, Rect};
use sparsha_input::FocusManager;
use sparsha_layout::{ComputedLayout, LayoutTree, WidgetId};
use sparsha_render::DrawList;
use sparsha_text::{TextLayoutAlignment, TextLayoutOptions, TextStyle, TextSystem, TextWrap};
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Runtime-owned storage for component-style build state.
#[doc(hidden)]
pub trait BuildStateStore {
    fn mark_path_used(&mut self, path: &[usize]);
    fn take_boxed_state(&mut self, path: &[usize]) -> Option<Box<dyn Any>>;
    fn store_boxed_state(&mut self, path: Vec<usize>, state: Box<dyn Any>);
}

type BuildStateMarkPathUsedFn = unsafe fn(*mut (), &[usize]);
type BuildStateTakeBoxedStateFn = unsafe fn(*mut (), &[usize]) -> Option<Box<dyn Any>>;
type BuildStateStoreBoxedStateFn = unsafe fn(*mut (), Vec<usize>, Box<dyn Any>);

#[derive(Clone, Copy)]
struct BuildStateStoreOps {
    ptr: *mut (),
    mark_path_used: BuildStateMarkPathUsedFn,
    take_boxed_state: BuildStateTakeBoxedStateFn,
    store_boxed_state: BuildStateStoreBoxedStateFn,
}

#[derive(Default)]
struct ResourceEntry {
    root: Option<Box<dyn Any>>,
    contexts: Vec<Box<dyn Any>>,
}

impl ResourceEntry {
    fn set_root(&mut self, value: Box<dyn Any>) {
        self.root = Some(value);
    }

    fn push_context(&mut self, value: Box<dyn Any>) {
        self.contexts.push(value);
    }

    fn pop_context(&mut self) -> bool {
        self.contexts.pop().is_some()
    }

    fn context<T: Clone + 'static>(&self) -> Option<T> {
        self.contexts
            .last()
            .and_then(|value| value.downcast_ref::<T>())
            .cloned()
    }

    fn resource<T: Clone + 'static>(&self) -> Option<T> {
        self.context::<T>().or_else(|| {
            self.root
                .as_ref()
                .and_then(|value| value.downcast_ref::<T>())
                .cloned()
        })
    }

    fn is_empty(&self) -> bool {
        self.root.is_none() && self.contexts.is_empty()
    }
}

/// Context for rebuilding dynamic widget children.
#[derive(Default)]
pub struct BuildContext {
    path: Vec<usize>,
    theme: Option<Theme>,
    resources: HashMap<TypeId, ResourceEntry>,
    state_store: Option<BuildStateStoreOps>,
}

impl BuildContext {
    /// Set the logical widget path currently being rebuilt.
    #[doc(hidden)]
    pub fn set_path(&mut self, path: &[usize]) {
        self.path.clear();
        self.path.extend_from_slice(path);
    }

    /// Return the current logical widget path.
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    /// Set the resolved theme for this rebuild pass.
    #[doc(hidden)]
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = Some(theme);
    }

    /// Return the resolved theme for the current rebuild pass.
    pub fn theme(&self) -> Theme {
        self.theme.clone().unwrap_or_else(crate::current_theme)
    }

    /// Insert or replace a root typed resource for rebuild-time consumers.
    #[doc(hidden)]
    pub fn insert_resource<T: 'static>(&mut self, value: T) {
        self.resources
            .entry(TypeId::of::<T>())
            .or_default()
            .set_root(Box::new(value));
    }

    /// Push a subtree-scoped typed context value for rebuild-time consumers.
    #[doc(hidden)]
    pub fn push_context<T: 'static>(&mut self, value: T) {
        self.resources
            .entry(TypeId::of::<T>())
            .or_default()
            .push_context(Box::new(value));
    }

    /// Pop the nearest subtree-scoped typed context value.
    #[doc(hidden)]
    pub fn pop_context<T: 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        let Some(entry) = self.resources.get_mut(&type_id) else {
            return;
        };
        let popped = entry.pop_context();
        debug_assert!(popped, "attempted to pop missing scoped context");
        if entry.is_empty() {
            self.resources.remove(&type_id);
        }
    }

    /// Fetch the nearest cloned provider-scoped context value for this subtree scope.
    pub fn context<T: Clone + 'static>(&self) -> Option<T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(ResourceEntry::context::<T>)
    }

    /// Fetch the nearest cloned rebuild-time resource for this subtree scope.
    ///
    /// Provider-scoped values shadow root runtime resources of the same type.
    pub fn resource<T: Clone + 'static>(&self) -> Option<T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(ResourceEntry::resource::<T>)
    }

    /// Attach runtime-owned component state storage.
    #[doc(hidden)]
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `store` outlives this `BuildContext` use
    /// and is not mutably aliased while the context may access it.
    pub unsafe fn set_state_store<T: BuildStateStore>(&mut self, store: &mut T) {
        unsafe fn mark_path_used<T: BuildStateStore>(ptr: *mut (), path: &[usize]) {
            (&mut *(ptr as *mut T)).mark_path_used(path);
        }

        unsafe fn take_boxed_state<T: BuildStateStore>(
            ptr: *mut (),
            path: &[usize],
        ) -> Option<Box<dyn Any>> {
            (&mut *(ptr as *mut T)).take_boxed_state(path)
        }

        unsafe fn store_boxed_state<T: BuildStateStore>(
            ptr: *mut (),
            path: Vec<usize>,
            state: Box<dyn Any>,
        ) {
            (&mut *(ptr as *mut T)).store_boxed_state(path, state);
        }

        self.state_store = Some(BuildStateStoreOps {
            ptr: store as *mut T as *mut (),
            mark_path_used: mark_path_used::<T>,
            take_boxed_state: take_boxed_state::<T>,
            store_boxed_state: store_boxed_state::<T>,
        });
    }

    /// Remove and return the boxed component state for the current path.
    #[doc(hidden)]
    pub fn take_boxed_state(&mut self) -> Option<Box<dyn Any>> {
        let path = self.path.clone();
        let store = self.state_store?;
        // SAFETY: the build pass owns the store for the lifetime of the context.
        unsafe {
            (store.mark_path_used)(store.ptr, &path);
            (store.take_boxed_state)(store.ptr, &path)
        }
    }

    /// Store boxed component state for the current path.
    #[doc(hidden)]
    pub fn store_boxed_state(&mut self, state: Box<dyn Any>) {
        let path = self.path.clone();
        let Some(store) = self.state_store else {
            return;
        };
        // SAFETY: the build pass owns the store for the lifetime of the context.
        unsafe {
            (store.mark_path_used)(store.ptr, &path);
            (store.store_boxed_state)(store.ptr, path, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BuildContext;

    #[test]
    fn root_resource_lookup_works() {
        let mut ctx = BuildContext::default();
        ctx.insert_resource(String::from("root"));

        assert_eq!(ctx.resource::<String>().as_deref(), Some("root"));
        assert_eq!(ctx.context::<String>(), None);
    }

    #[test]
    fn nested_context_scope_shadows_parent() {
        let mut ctx = BuildContext::default();
        ctx.push_context(String::from("root"));
        ctx.push_context(String::from("child"));

        assert_eq!(ctx.context::<String>().as_deref(), Some("child"));
        assert_eq!(ctx.resource::<String>().as_deref(), Some("child"));
    }

    #[test]
    fn pop_context_restores_parent_value() {
        let mut ctx = BuildContext::default();
        ctx.push_context(String::from("root"));
        ctx.push_context(String::from("child"));
        ctx.pop_context::<String>();

        assert_eq!(ctx.context::<String>().as_deref(), Some("root"));
        assert_eq!(ctx.resource::<String>().as_deref(), Some("root"));
    }

    #[test]
    fn sibling_subtrees_do_not_leak_context_values() {
        let mut ctx = BuildContext::default();
        ctx.push_context(String::from("root"));

        ctx.push_context(String::from("left"));
        assert_eq!(ctx.context::<String>().as_deref(), Some("left"));
        assert_eq!(ctx.resource::<String>().as_deref(), Some("left"));
        ctx.pop_context::<String>();

        assert_eq!(ctx.context::<String>().as_deref(), Some("root"));
        assert_eq!(ctx.resource::<String>().as_deref(), Some("root"));

        ctx.push_context(String::from("right"));
        assert_eq!(ctx.context::<String>().as_deref(), Some("right"));
        assert_eq!(ctx.resource::<String>().as_deref(), Some("right"));
        ctx.pop_context::<String>();

        assert_eq!(ctx.context::<String>().as_deref(), Some("root"));
        assert_eq!(ctx.resource::<String>().as_deref(), Some("root"));
    }

    #[test]
    fn context_lookup_ignores_root_runtime_resources() {
        let mut ctx = BuildContext::default();
        ctx.insert_resource(String::from("root"));

        assert_eq!(ctx.context::<String>(), None);
        assert_eq!(ctx.resource::<String>().as_deref(), Some("root"));
    }

    #[test]
    fn pop_context_does_not_remove_root_resource() {
        let mut ctx = BuildContext::default();
        ctx.insert_resource(String::from("root"));
        ctx.push_context(String::from("child"));
        ctx.pop_context::<String>();

        assert_eq!(ctx.context::<String>(), None);
        assert_eq!(ctx.resource::<String>().as_deref(), Some("root"));
    }
}

/// Context for layout measurement.
pub struct LayoutContext<'a> {
    /// The text system for measuring text.
    pub text: &'a mut TextSystem,
    /// Available width constraint.
    pub max_width: Option<f32>,
    /// Available height constraint.
    pub max_height: Option<f32>,
}

impl<'a> LayoutContext<'a> {
    /// Measure text with the current constraints.
    pub fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        self.text.measure(text, style, self.max_width)
    }

    /// Measure text with explicit layout options.
    pub fn measure_text_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        wrap: TextWrap,
        alignment: TextLayoutAlignment,
        max_lines: Option<usize>,
    ) -> (f32, f32) {
        self.text.measure_with_options(
            text,
            style,
            TextLayoutOptions::new()
                .with_max_width(self.max_width)
                .with_wrap(wrap)
                .with_alignment(alignment)
                .with_max_lines(max_lines),
        )
    }
}

/// Context for painting widgets.
pub struct PaintContext<'a> {
    /// The draw list to paint to.
    pub draw_list: &'a mut DrawList,
    /// The computed layout for this widget.
    pub layout: ComputedLayout,
    /// The layout tree for querying child layouts.
    pub layout_tree: &'a LayoutTree,
    /// The focus manager (for focus state).
    pub focus: &'a FocusManager,
    /// Current widget ID.
    pub widget_id: WidgetId,
    /// Scale factor for HiDPI.
    pub scale_factor: f32,
    /// The text system for shaping text.
    pub text_system: &'a mut TextSystem,
    /// Elapsed time in seconds (for animations like cursor blinking).
    pub elapsed_time: f32,
    /// Commands requested during paint.
    pub commands: &'a mut PaintCommands,
}

impl<'a> PaintContext<'a> {
    /// Get the widget's bounds.
    pub fn bounds(&self) -> Rect {
        self.layout.bounds
    }

    /// Check if this widget has keyboard focus.
    pub fn has_focus(&self) -> bool {
        self.focus.has_focus(self.widget_id)
    }

    /// Draw a filled rectangle.
    /// Bounds are in physical pixels.
    pub fn fill_rect(&mut self, bounds: Rect, color: Color) {
        self.draw_list.rect(bounds, color);
    }

    /// Draw a rounded rectangle.
    /// Bounds and radius are in physical pixels.
    pub fn fill_rounded_rect(&mut self, bounds: Rect, color: Color, radius: f32) {
        // Scale radius for HiDPI
        let scaled_radius = radius * self.scale_factor;
        self.draw_list.rounded_rect(bounds, color, scaled_radius);
    }

    /// Draw a rectangle with a border.
    /// Bounds, radius, and border_width are in physical pixels.
    pub fn fill_bordered_rect(
        &mut self,
        bounds: Rect,
        color: Color,
        radius: f32,
        border_width: f32,
        border_color: Color,
    ) {
        // Scale radius and border for HiDPI
        let scaled_radius = radius * self.scale_factor;
        let scaled_border = border_width * self.scale_factor;
        self.draw_list
            .bordered_rect(bounds, color, scaled_radius, scaled_border, border_color);
    }

    /// Draw a line segment.
    /// Coordinates and thickness are in physical pixels.
    pub fn stroke_line(
        &mut self,
        start: sparsha_core::Point,
        end: sparsha_core::Point,
        thickness: f32,
        color: Color,
    ) {
        let scaled_thickness = thickness * self.scale_factor;
        self.draw_list
            .line((start.x, start.y), (end.x, end.y), scaled_thickness, color);
    }

    /// Push a clip rectangle.
    pub fn push_clip(&mut self, bounds: Rect) {
        self.draw_list.push_clip(bounds);
    }

    /// Pop the clip rectangle.
    pub fn pop_clip(&mut self) {
        self.draw_list.pop_clip();
    }

    /// Push a translation offset for subsequent draw commands.
    /// The offset is in physical pixels.
    pub fn push_translation(&mut self, offset: (f32, f32)) {
        self.draw_list.push_translation(offset);
    }

    /// Pop the current translation offset.
    pub fn pop_translation(&mut self) {
        self.draw_list.pop_translation();
    }

    /// Draw text at the specified position.
    ///
    /// The text run is emitted into the draw list and consumed by the active renderer.
    /// Coordinates are in physical pixels.
    pub fn draw_text(&mut self, text: &str, style: &TextStyle, x: f32, y: f32) {
        if text.is_empty() {
            return;
        }

        // Scale font size for HiDPI rendering
        let scaled_style = TextStyle {
            font_size: style.font_size * self.scale_factor,
            ..style.clone()
        };
        self.draw_list.text_run(text.to_owned(), scaled_style, x, y);
    }

    /// Draw block text inside the given bounds using the provided alignment and optional clamp.
    pub fn draw_text_block(
        &mut self,
        text: &str,
        style: &TextStyle,
        bounds: Rect,
        wrap: TextWrap,
        alignment: TextLayoutAlignment,
        max_lines: Option<usize>,
    ) {
        if text.is_empty() {
            return;
        }

        let scaled_style = TextStyle {
            font_size: style.font_size * self.scale_factor,
            ..style.clone()
        };
        self.draw_list.text_run_layout(
            text.to_owned(),
            scaled_style,
            bounds.x,
            bounds.y,
            Some(bounds.width.max(0.0)),
            alignment,
            max_lines,
            wrap,
        );
    }

    /// Draw text centered within the given bounds.
    ///
    /// The text is horizontally and vertically centered within the bounds.
    /// Bounds are in physical pixels.
    pub fn draw_text_centered(&mut self, text: &str, style: &TextStyle, bounds: Rect) {
        if text.is_empty() {
            return;
        }

        // Measure text at scaled size to get dimensions
        let (text_width, text_height) = self.measure_text(text, style);

        // Calculate centered position
        let x = bounds.x + (bounds.width - text_width) / 2.0;
        let y = bounds.y + (bounds.height - text_height) / 2.0;

        self.draw_text(text, style, x, y);
    }

    /// Draw text left-aligned within the given bounds, vertically centered.
    ///
    /// Useful for text inputs and labels. Bounds are in physical pixels.
    pub fn draw_text_aligned(
        &mut self,
        text: &str,
        style: &TextStyle,
        bounds: Rect,
        padding_left: f32,
    ) {
        if text.is_empty() {
            return;
        }

        // Measure text at scaled size to get dimensions
        let (_text_width, text_height) = self.measure_text(text, style);

        // Calculate position: left-aligned with padding, vertically centered
        // Padding is also in physical pixels since bounds are
        let x = bounds.x + padding_left;
        let y = bounds.y + (bounds.height - text_height) / 2.0;

        self.draw_text(text, style, x, y);
    }

    /// Measure text dimensions without drawing.
    /// Returns dimensions in physical pixels (scaled by scale_factor).
    pub fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        // Scale font size for HiDPI measurement
        let scaled_style = TextStyle {
            font_size: style.font_size * self.scale_factor,
            ..style.clone()
        };
        self.text_system.measure(text, &scaled_style, None)
    }

    /// Measure text with explicit width/alignment/clamp options.
    pub fn measure_text_layout(
        &mut self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f32>,
        wrap: TextWrap,
        alignment: TextLayoutAlignment,
        max_lines: Option<usize>,
    ) -> (f32, f32) {
        let scaled_style = TextStyle {
            font_size: style.font_size * self.scale_factor,
            ..style.clone()
        };
        self.text_system.measure_with_options(
            text,
            &scaled_style,
            TextLayoutOptions::new()
                .with_max_width(max_width)
                .with_wrap(wrap)
                .with_alignment(alignment)
                .with_max_lines(max_lines),
        )
    }

    /// Request another animation frame after this paint.
    pub fn request_next_frame(&mut self) {
        self.commands.request_next_frame = true;
    }

    /// Request a relayout after this paint.
    pub fn request_layout(&mut self) {
        self.commands.request_layout = true;
    }
}

/// Commands emitted during painting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintCommands {
    pub request_next_frame: bool,
    pub request_layout: bool,
}

impl PaintCommands {
    pub fn merge(&mut self, other: PaintCommands) {
        self.request_next_frame |= other.request_next_frame;
        self.request_layout |= other.request_layout;
    }
}

/// Commands emitted by a widget during event handling.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventCommands {
    pub stop_propagation: bool,
    pub capture_pointer: bool,
    pub release_pointer: bool,
    pub request_focus: bool,
    pub clear_focus: bool,
    pub request_layout: bool,
    pub request_paint: bool,
    pub clipboard_write: Option<String>,
}

impl EventCommands {
    pub fn merge(&mut self, other: EventCommands) {
        self.stop_propagation |= other.stop_propagation;
        self.capture_pointer |= other.capture_pointer;
        self.release_pointer |= other.release_pointer;
        self.request_focus |= other.request_focus;
        self.clear_focus |= other.clear_focus;
        self.request_layout |= other.request_layout;
        self.request_paint |= other.request_paint;
        if other.clipboard_write.is_some() {
            self.clipboard_write = other.clipboard_write;
        }
    }
}

/// Context for handling events.
pub struct EventContext<'a> {
    /// The computed layout for this widget.
    pub layout: ComputedLayout,
    /// The layout tree for hit testing children.
    pub layout_tree: &'a LayoutTree,
    /// Focus manager.
    pub focus: &'a mut FocusManager,
    /// Current widget ID.
    pub widget_id: WidgetId,
    /// Whether this widget has pointer capture.
    pub has_capture: bool,
    /// Commands requested by this event handler.
    pub commands: EventCommands,
}

impl<'a> EventContext<'a> {
    /// Get the widget's bounds.
    pub fn bounds(&self) -> Rect {
        self.layout.bounds
    }

    /// Check if this widget has keyboard focus.
    pub fn has_focus(&self) -> bool {
        self.focus.has_focus(self.widget_id)
    }

    /// Request keyboard focus for this widget.
    pub fn request_focus(&mut self) {
        self.focus.set_focus(self.widget_id);
        self.commands.request_focus = true;
    }

    /// Release keyboard focus.
    pub fn release_focus(&mut self) {
        if self.has_focus() {
            self.focus.clear_focus();
            self.commands.clear_focus = true;
        }
    }

    /// Check if a point is inside this widget's bounds.
    pub fn contains(&self, pos: glam::Vec2) -> bool {
        self.layout.bounds.contains(pos)
    }

    /// Convert a point to local coordinates.
    pub fn to_local(&self, pos: glam::Vec2) -> glam::Vec2 {
        glam::Vec2::new(pos.x - self.layout.bounds.x, pos.y - self.layout.bounds.y)
    }

    /// Stop further event propagation.
    pub fn stop_propagation(&mut self) {
        self.commands.stop_propagation = true;
    }

    /// Request pointer capture.
    pub fn capture_pointer(&mut self) {
        self.commands.capture_pointer = true;
        self.commands.request_paint = true;
        self.commands.stop_propagation = true;
    }

    /// Request pointer capture release.
    pub fn release_pointer(&mut self) {
        self.commands.release_pointer = true;
        self.commands.request_paint = true;
        self.commands.stop_propagation = true;
    }

    /// Request a repaint.
    pub fn request_paint(&mut self) {
        self.commands.request_paint = true;
    }

    /// Request relayout + repaint.
    pub fn request_layout(&mut self) {
        self.commands.request_layout = true;
        self.commands.request_paint = true;
    }

    /// Request writing text to the platform clipboard.
    pub fn write_clipboard(&mut self, text: impl Into<String>) {
        self.commands.clipboard_write = Some(text.into());
    }
}
