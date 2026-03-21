//! List widget for dynamic collections of child widgets.

use crate::{
    current_theme,
    scroll_model::{ScrollAxes, ScrollModel},
    AccessibilityInfo, AccessibilityRole, EventContext, IntoWidget, PaintContext, Widget,
};
use sparsh_core::Rect as CoreRect;
use sparsh_input::InputEvent;
use sparsh_layout::WidgetId;
use std::cell::Cell;
use std::collections::HashMap;
use taffy::prelude::*;

const TOP_SPACER_KEY: usize = usize::MAX - 1;
const BOTTOM_SPACER_KEY: usize = usize::MAX;

type ListItemBuilder = Box<dyn Fn(usize) -> Box<dyn Widget>>;

/// Layout direction for list items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListDirection {
    /// Stack items vertically.
    #[default]
    Vertical,
    /// Stack items horizontally.
    Horizontal,
}

enum ListMode {
    Owned(Vec<Box<dyn Widget>>),
    Virtualized(VirtualizedListState),
}

struct VirtualizedListState {
    item_count: usize,
    item_extent: f32,
    overscan: usize,
    builder: ListItemBuilder,
    model: ScrollModel,
    viewport_extent: Cell<f32>,
    child_keys: Vec<usize>,
    children: Vec<Box<dyn Widget>>,
    realized_range: std::ops::Range<usize>,
}

impl VirtualizedListState {
    fn new(item_count: usize, item_extent: f32, builder: ListItemBuilder) -> Self {
        Self {
            item_count,
            item_extent: item_extent.max(1.0),
            overscan: 2,
            builder,
            model: ScrollModel::default(),
            viewport_extent: Cell::new(0.0),
            child_keys: Vec::new(),
            children: Vec::new(),
            realized_range: 0..0,
        }
    }

    fn content_size(&self, direction: ListDirection, viewport_cross_extent: f32) -> (f32, f32) {
        let total_extent = self.item_count as f32 * self.item_extent;
        match direction {
            ListDirection::Vertical => (viewport_cross_extent.max(0.0), total_extent),
            ListDirection::Horizontal => (total_extent, viewport_cross_extent.max(0.0)),
        }
    }

    fn axes(&self, direction: ListDirection) -> ScrollAxes {
        match direction {
            ListDirection::Vertical => ScrollAxes::new(false, true),
            ListDirection::Horizontal => ScrollAxes::new(true, false),
        }
    }

    fn main_offset(&self, direction: ListDirection) -> f32 {
        match direction {
            ListDirection::Vertical => self.model.offset().1,
            ListDirection::Horizontal => self.model.offset().0,
        }
    }

    fn viewport_hint(&self) -> f32 {
        self.viewport_extent.get().max(self.item_extent)
    }

    fn update_realized_children(&mut self, direction: ListDirection) {
        let first_visible = (self.main_offset(direction) / self.item_extent).floor() as usize;
        let last_visible = ((self.main_offset(direction) + self.viewport_hint()) / self.item_extent)
            .ceil() as usize;
        let start = first_visible
            .saturating_sub(self.overscan)
            .min(self.item_count);
        let end = (last_visible + self.overscan).min(self.item_count);

        let old_keys = std::mem::take(&mut self.child_keys);
        let old_children = std::mem::take(&mut self.children);
        let mut realized: HashMap<usize, Box<dyn Widget>> = old_keys
            .into_iter()
            .zip(old_children)
            .filter_map(|(key, child)| {
                (![TOP_SPACER_KEY, BOTTOM_SPACER_KEY].contains(&key)).then_some((key, child))
            })
            .collect();

        self.child_keys.clear();
        self.children.clear();

        if start > 0 {
            self.child_keys.push(TOP_SPACER_KEY);
            self.children.push(Box::new(ListSpacer::new(
                direction,
                start as f32 * self.item_extent,
            )));
        }

        for index in start..end {
            self.child_keys.push(index);
            let child = realized
                .remove(&index)
                .unwrap_or_else(|| (self.builder)(index));
            self.children.push(child);
        }

        if end < self.item_count {
            self.child_keys.push(BOTTOM_SPACER_KEY);
            self.children.push(Box::new(ListSpacer::new(
                direction,
                (self.item_count - end) as f32 * self.item_extent,
            )));
        }

        self.realized_range = start..end;
    }

    fn child_slot_for_key(&self, key: usize) -> Option<usize> {
        self.child_keys
            .iter()
            .position(|candidate| *candidate == key)
    }
}

struct ListSpacer {
    id: WidgetId,
    direction: ListDirection,
    extent: f32,
}

impl ListSpacer {
    fn new(direction: ListDirection, extent: f32) -> Self {
        Self {
            id: WidgetId::default(),
            direction,
            extent,
        }
    }
}

impl Widget for ListSpacer {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        match self.direction {
            ListDirection::Vertical => Style {
                size: Size {
                    width: percent(1.0),
                    height: length(self.extent),
                },
                min_size: Size {
                    width: percent(1.0),
                    height: length(self.extent),
                },
                ..Default::default()
            },
            ListDirection::Horizontal => Style {
                size: Size {
                    width: length(self.extent),
                    height: percent(1.0),
                },
                min_size: Size {
                    width: length(self.extent),
                    height: percent(1.0),
                },
                ..Default::default()
            },
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}
}

/// A dynamic list container that owns item widgets or realizes them on demand.
pub struct List {
    id: WidgetId,
    mode: ListMode,
    direction: ListDirection,
    style: Style,
}

impl List {
    /// Create a new empty vertical list.
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            mode: ListMode::Owned(Vec::new()),
            direction: ListDirection::Vertical,
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        }
    }

    /// Create a new fixed-extent virtualized list.
    pub fn virtualized(
        item_count: usize,
        item_extent: f32,
        builder: impl Fn(usize) -> Box<dyn Widget> + 'static,
    ) -> Self {
        Self {
            id: WidgetId::default(),
            mode: ListMode::Virtualized(VirtualizedListState::new(
                item_count,
                item_extent,
                Box::new(builder),
            )),
            direction: ListDirection::Vertical,
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                overflow: taffy::Point {
                    x: taffy::Overflow::Hidden,
                    y: taffy::Overflow::Hidden,
                },
                ..Default::default()
            },
        }
    }

    /// Set the list direction.
    pub fn direction(mut self, direction: ListDirection) -> Self {
        self.direction = direction;
        self.style.flex_direction = match direction {
            ListDirection::Vertical => FlexDirection::Column,
            ListDirection::Horizontal => FlexDirection::Row,
        };
        self
    }

    /// Make list vertical.
    pub fn vertical(self) -> Self {
        self.direction(ListDirection::Vertical)
    }

    /// Make list horizontal.
    pub fn horizontal(self) -> Self {
        self.direction(ListDirection::Horizontal)
    }

    /// Set item gap.
    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = Size {
            width: length(gap),
            height: length(gap),
        };
        self
    }

    /// Set virtualization overscan count.
    pub fn overscan(mut self, count: usize) -> Self {
        if let ListMode::Virtualized(state) = &mut self.mode {
            state.overscan = count;
        }
        self
    }

    /// Set padding on all sides.
    pub fn padding(mut self, all: f32) -> Self {
        self.style.padding = taffy::Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        self
    }

    /// Fill available width.
    pub fn fill_width(mut self) -> Self {
        self.style.size.width = percent(1.0);
        self
    }

    /// Fill available height.
    pub fn fill_height(mut self) -> Self {
        self.style.size.height = percent(1.0);
        self
    }

    /// Fill width and height.
    pub fn fill(mut self) -> Self {
        self.style.size = Size {
            width: percent(1.0),
            height: percent(1.0),
        };
        self
    }

    /// Set list items.
    pub fn with_items(mut self, items: Vec<Box<dyn Widget>>) -> Self {
        self.mode = ListMode::Owned(items);
        self
    }

    /// Replace all items at runtime.
    pub fn set_items(&mut self, items: Vec<Box<dyn Widget>>) {
        self.mode = ListMode::Owned(items);
    }

    fn ensure_owned_items(&mut self) -> &mut Vec<Box<dyn Widget>> {
        if let ListMode::Virtualized(state) = &self.mode {
            let items: Vec<_> = (0..state.item_count)
                .map(|index| (state.builder)(index))
                .collect();
            self.mode = ListMode::Owned(items);
        }

        let ListMode::Owned(items) = &mut self.mode else {
            unreachable!()
        };
        items
    }

    /// Append an item widget.
    pub fn push_item(&mut self, widget: impl IntoWidget) {
        self.ensure_owned_items().push(widget.into_widget());
    }

    /// Append a boxed item widget.
    pub fn push_boxed_item(&mut self, widget: Box<dyn Widget>) {
        self.ensure_owned_items().push(widget);
    }

    /// Remove the item at index.
    pub fn remove_item(&mut self, index: usize) -> Option<Box<dyn Widget>> {
        let items = self.ensure_owned_items();
        if index < items.len() {
            Some(items.remove(index))
        } else {
            None
        }
    }

    /// Remove all items.
    pub fn clear(&mut self) {
        self.ensure_owned_items().clear();
    }

    /// Get number of items.
    pub fn len(&self) -> usize {
        match &self.mode {
            ListMode::Owned(items) => items.len(),
            ListMode::Virtualized(state) => state.item_count,
        }
    }

    /// Check if list has no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn virtualized_style(&self) -> Style {
        let mut style = self.style.clone();
        style.gap = Size {
            width: length(0.0),
            height: length(0.0),
        };
        style
    }

    fn resolved_scrollbar(&self) -> crate::ScrollbarStyle {
        let theme = current_theme();
        crate::ScrollbarStyle {
            track_color: theme.colors.surface_variant,
            thumb_color: theme.colors.border,
            thumb_hover_color: theme.colors.primary_hovered,
            width: theme.controls.scrollbar_thickness,
            corner_radius: theme.radii.md,
        }
    }

    fn main_extent(bounds: CoreRect, direction: ListDirection) -> f32 {
        match direction {
            ListDirection::Vertical => bounds.height,
            ListDirection::Horizontal => bounds.width,
        }
    }

    fn cross_extent(bounds: CoreRect, direction: ListDirection) -> f32 {
        match direction {
            ListDirection::Vertical => bounds.width,
            ListDirection::Horizontal => bounds.height,
        }
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for List {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        match self.mode {
            ListMode::Owned(_) => self.style.clone(),
            ListMode::Virtualized(_) => self.virtualized_style(),
        }
    }

    fn is_scroll_container(&self) -> bool {
        matches!(self.mode, ListMode::Virtualized(_))
    }

    fn child_event_offset(&self) -> glam::Vec2 {
        match &self.mode {
            ListMode::Owned(_) => glam::Vec2::ZERO,
            ListMode::Virtualized(state) => {
                let (x, y) = state.model.offset();
                glam::vec2(x, y)
            }
        }
    }

    fn child_path_key(&self, child_position: usize) -> usize {
        match &self.mode {
            ListMode::Owned(_) => child_position,
            ListMode::Virtualized(state) => state
                .child_keys
                .get(child_position)
                .copied()
                .unwrap_or(child_position),
        }
    }

    fn child_slot_for_path_key(&self, key: usize) -> Option<usize> {
        match &self.mode {
            ListMode::Owned(_) => Some(key),
            ListMode::Virtualized(state) => state.child_slot_for_key(key),
        }
    }

    fn rebuild(&mut self, _ctx: &mut crate::BuildContext) {
        if let ListMode::Virtualized(state) = &mut self.mode {
            state.update_realized_children(self.direction);
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let ListMode::Virtualized(state) = &self.mode else {
            return;
        };

        let bounds = ctx.bounds();
        let scale_factor = ctx.scale_factor.max(1.0);
        let logical_bounds = CoreRect::new(
            bounds.x / scale_factor,
            bounds.y / scale_factor,
            bounds.width / scale_factor,
            bounds.height / scale_factor,
        );
        let viewport_extent = Self::main_extent(logical_bounds, self.direction);
        if (state.viewport_extent.get() - viewport_extent).abs() > f32::EPSILON {
            state.viewport_extent.set(viewport_extent);
            ctx.request_layout();
        }

        ctx.push_clip(bounds);
        let (offset_x, offset_y) = state.model.offset();
        ctx.push_translation((-offset_x * scale_factor, -offset_y * scale_factor));
    }

    fn paint_after_children(&self, ctx: &mut PaintContext) {
        let ListMode::Virtualized(state) = &self.mode else {
            return;
        };

        let bounds = ctx.bounds();
        let scale_factor = ctx.scale_factor.max(1.0);
        let logical_bounds = CoreRect::new(
            bounds.x / scale_factor,
            bounds.y / scale_factor,
            bounds.width / scale_factor,
            bounds.height / scale_factor,
        );
        let content_size = state.content_size(
            self.direction,
            Self::cross_extent(logical_bounds, self.direction),
        );
        let scrollbar_style = self.resolved_scrollbar();
        let scrollbars = state.model.scrollbars(
            logical_bounds,
            content_size,
            scrollbar_style.width,
            state.axes(self.direction),
        );

        ctx.pop_translation();
        ctx.pop_clip();

        if let Some(track) = scrollbars.vertical_track {
            ctx.fill_rounded_rect(
                scale_rect(track, scale_factor),
                scrollbar_style.track_color,
                scrollbar_style.corner_radius,
            );
        }
        if let Some(track) = scrollbars.horizontal_track {
            ctx.fill_rounded_rect(
                scale_rect(track, scale_factor),
                scrollbar_style.track_color,
                scrollbar_style.corner_radius,
            );
        }
        if let Some(thumb) = scrollbars.vertical_thumb {
            let thumb_color =
                if state.model.hover_axis().is_some() || state.model.dragging_axis().is_some() {
                    scrollbar_style.thumb_hover_color
                } else {
                    scrollbar_style.thumb_color
                };
            ctx.fill_rounded_rect(
                scale_rect(thumb, scale_factor),
                thumb_color,
                scrollbar_style.corner_radius,
            );
        }
        if let Some(thumb) = scrollbars.horizontal_thumb {
            let thumb_color =
                if state.model.hover_axis().is_some() || state.model.dragging_axis().is_some() {
                    scrollbar_style.thumb_hover_color
                } else {
                    scrollbar_style.thumb_color
                };
            ctx.fill_rounded_rect(
                scale_rect(thumb, scale_factor),
                thumb_color,
                scrollbar_style.corner_radius,
            );
        }
    }

    fn event(&mut self, ctx: &mut EventContext, event: &sparsh_input::InputEvent) {
        let viewport = ctx.bounds();
        let scrollbar_width = self.resolved_scrollbar().width;
        let ListMode::Virtualized(state) = &mut self.mode else {
            return;
        };

        let content_size =
            state.content_size(self.direction, Self::cross_extent(viewport, self.direction));
        state
            .model
            .clamp(viewport, content_size, state.axes(self.direction));

        match event {
            InputEvent::Scroll {
                pos,
                delta,
                modifiers,
            } if ctx.contains(*pos)
                && state.model.scroll_by(
                    viewport,
                    content_size,
                    state.axes(self.direction),
                    *delta,
                    *modifiers,
                ) =>
            {
                ctx.stop_propagation();
                ctx.request_layout();
            }
            InputEvent::PointerDown { pos, .. } if ctx.contains(*pos) => {
                let result = state.model.pointer_down(
                    *pos,
                    viewport,
                    content_size,
                    scrollbar_width,
                    state.axes(self.direction),
                );
                if result.capture_pointer {
                    ctx.capture_pointer();
                }
                if result.consume {
                    ctx.stop_propagation();
                }
                if result.changed_offset {
                    ctx.request_layout();
                } else if result.changed_visuals {
                    ctx.request_paint();
                }
            }
            InputEvent::PointerMove { pos } => {
                let result = state.model.pointer_move(
                    *pos,
                    viewport,
                    content_size,
                    scrollbar_width,
                    state.axes(self.direction),
                );
                if result.consume {
                    ctx.stop_propagation();
                }
                if result.changed_offset {
                    ctx.request_layout();
                } else if result.changed_visuals {
                    ctx.request_paint();
                }
            }
            InputEvent::PointerUp { pos, .. } => {
                let result = state.model.pointer_up(*pos);
                if result.release_pointer {
                    ctx.release_pointer();
                }
                if result.consume {
                    ctx.stop_propagation();
                }
                if result.changed_visuals {
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        match &self.mode {
            ListMode::Owned(items) => items,
            ListMode::Virtualized(state) => &state.children,
        }
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        match &mut self.mode {
            ListMode::Owned(items) => items,
            ListMode::Virtualized(state) => &mut state.children,
        }
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        Some(AccessibilityInfo::new(AccessibilityRole::List))
    }
}

fn scale_rect(rect: CoreRect, scale_factor: f32) -> CoreRect {
    CoreRect::new(
        rect.x * scale_factor,
        rect.y * scale_factor,
        rect.width * scale_factor,
        rect.height * scale_factor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, Text};
    use sparsh_input::{FocusManager, Modifiers};
    use sparsh_layout::LayoutTree;

    #[test]
    fn list_runtime_item_operations_work() {
        let mut list = List::new().vertical();
        assert!(list.is_empty());

        list.push_item(Text::new("A"));
        list.push_item(Text::new("B"));
        assert_eq!(list.len(), 2);

        let removed = list.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(list.len(), 1);

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn list_set_items_replaces_children() {
        let mut list = List::new();
        list.set_items(vec![Box::new(Text::new("One")), Box::new(Text::new("Two"))]);
        assert_eq!(list.children().len(), 2);

        list.set_items(vec![Box::new(Text::new("Only"))]);
        assert_eq!(list.children_mut().len(), 1);
    }

    #[test]
    fn virtualized_list_realizes_visible_range_with_overscan() {
        let mut list = List::virtualized(100, 20.0, |index| Box::new(Text::new(index.to_string())))
            .overscan(1)
            .vertical();
        if let ListMode::Virtualized(state) = &mut list.mode {
            state.viewport_extent.set(60.0);
            state.model.set_offset(0.0, 40.0);
            state.update_realized_children(ListDirection::Vertical);
            assert_eq!(state.realized_range, 1..6);
        } else {
            panic!("expected virtualized mode");
        }
    }

    #[test]
    fn virtualized_list_child_keys_track_logical_indices() {
        let mut list = List::virtualized(50, 24.0, |index| Box::new(Text::new(index.to_string())));
        if let ListMode::Virtualized(state) = &mut list.mode {
            state.viewport_extent.set(48.0);
            state.model.set_offset(0.0, 24.0);
            state.update_realized_children(ListDirection::Vertical);
            let keys = state.child_keys.clone();
            assert!(keys.contains(&1));
            assert!(keys.contains(&2));
            assert!(keys.contains(&3));
        } else {
            panic!("expected virtualized mode");
        }
    }

    #[test]
    fn virtualized_list_horizontal_scrolls_on_primary_axis() {
        let mut list = List::virtualized(30, 30.0, |index| Box::new(Text::new(index.to_string())))
            .horizontal();
        let layout_tree = LayoutTree::new();
        let mut focus = FocusManager::new();
        list.set_id(Default::default());
        let layout = crate::test_helpers::layout_bounds(0.0, 0.0, 120.0, 40.0);

        if let ListMode::Virtualized(state) = &mut list.mode {
            state.viewport_extent.set(120.0);
            state.update_realized_children(ListDirection::Horizontal);
        }

        let mut ctx = crate::test_helpers::mock_event_context(
            layout,
            &layout_tree,
            &mut focus,
            list.id(),
            false,
        );
        list.event(
            &mut ctx,
            &InputEvent::Scroll {
                pos: glam::vec2(20.0, 20.0),
                delta: glam::vec2(-2.0, 0.0),
                modifiers: Modifiers::default(),
            },
        );
        if let ListMode::Virtualized(state) = &list.mode {
            assert!(state.model.offset().0 > 0.0);
        }
    }

    #[test]
    fn mutating_virtualized_list_materializes_owned_mode() {
        let mut list = List::virtualized(3, 20.0, |index| {
            Box::new(Container::new().child(Text::new(format!("Row {index}"))))
        });
        list.push_item(Text::new("Extra"));
        assert_eq!(list.len(), 4);
        assert!(matches!(list.mode, ListMode::Owned(_)));
    }
}
