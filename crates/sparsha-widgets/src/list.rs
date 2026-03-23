//! List widget for dynamic collections of child widgets.

use crate::{
    current_theme, responsive_theme_controls,
    scroll_model::{ScrollAxes, ScrollModel},
    AccessibilityInfo, AccessibilityRole, EventContext, IntoWidget, PaintContext, Widget,
};
use bon::bon;
use sparsha_core::Rect as CoreRect;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
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

#[derive(Clone, Copy)]
struct VirtualizedListBuildState {
    model: ScrollModel,
}

impl List {
    /// Create an empty owned vertical list.
    pub fn empty() -> Self {
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
        Self::new_virtualized_with_builder(item_count, item_extent, Box::new(builder))
    }

    fn new_virtualized_with_builder(
        item_count: usize,
        item_extent: f32,
        builder: ListItemBuilder,
    ) -> Self {
        Self {
            id: WidgetId::default(),
            mode: ListMode::Virtualized(VirtualizedListState::new(
                item_count,
                item_extent,
                builder,
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
        let controls = responsive_theme_controls(&theme);
        crate::ScrollbarStyle {
            track_color: theme.colors.surface_variant,
            thumb_color: theme.colors.border,
            thumb_hover_color: theme.colors.primary_hovered,
            width: controls.scrollbar_thickness,
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

#[bon]
impl List {
    #[builder(
        start_fn(name = virtualized_builder, vis = "pub"),
        finish_fn(name = build, vis = "pub"),
        builder_type(name = VirtualizedListBuilder, vis = "pub"),
        state_mod(name = virtualized_list_builder, vis = "pub")
    )]
    fn virtualized_builder_init(
        item_count: usize,
        item_extent: f32,
        #[builder(with = |item_builder: impl Fn(usize) -> Box<dyn Widget> + 'static| Box::new(item_builder) as ListItemBuilder)]
        item_builder: ListItemBuilder,
        #[builder(default = ListDirection::Vertical)] direction: ListDirection,
        #[builder(default = 2)] overscan: usize,
        padding: Option<f32>,
        #[builder(default)] fill_width: bool,
        #[builder(default)] fill_height: bool,
        #[builder(default)] fill: bool,
    ) -> Self {
        let mut list = Self::new_virtualized_with_builder(item_count, item_extent, item_builder)
            .direction(direction)
            .overscan(overscan);
        if let Some(padding) = padding {
            list = list.padding(padding);
        }
        if fill_width {
            list = list.fill_width();
        }
        if fill_height {
            list = list.fill_height();
        }
        if fill {
            list = list.fill();
        }
        list
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

    fn rebuild(&mut self, ctx: &mut crate::BuildContext) {
        if let ListMode::Virtualized(state) = &mut self.mode {
            if let Some(saved) = ctx
                .take_boxed_state()
                .and_then(|state| state.downcast::<VirtualizedListBuildState>().ok())
                .map(|state| *state)
            {
                state.model = saved.model;
            }

            state.update_realized_children(self.direction);

            ctx.store_boxed_state(Box::new(VirtualizedListBuildState { model: state.model }));
        }
    }

    fn persist_build_state(&self, ctx: &mut crate::BuildContext) {
        if let ListMode::Virtualized(state) = &self.mode {
            ctx.store_boxed_state(Box::new(VirtualizedListBuildState { model: state.model }));
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

    fn event(&mut self, ctx: &mut EventContext, event: &sparsha_input::InputEvent) {
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
    use sparsha_input::{FocusManager, Modifiers};
    use sparsha_layout::LayoutTree;

    #[test]
    fn list_runtime_item_operations_work() {
        let mut list = List::empty().vertical();
        assert!(list.is_empty());

        list.push_item(Text::builder().content("A").build());
        list.push_item(Text::builder().content("B").build());
        assert_eq!(list.len(), 2);

        let removed = list.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(list.len(), 1);

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn list_set_items_replaces_children() {
        let mut list = List::empty();
        list.set_items(vec![
            Box::new(Text::builder().content("One").build()),
            Box::new(Text::builder().content("Two").build()),
        ]);
        assert_eq!(list.children().len(), 2);

        list.set_items(vec![Box::new(Text::builder().content("Only").build())]);
        assert_eq!(list.children_mut().len(), 1);
    }

    #[test]
    fn virtualized_list_realizes_visible_range_with_overscan() {
        let mut list = List::virtualized(100, 20.0, |index| {
            Box::new(Text::builder().content(index.to_string()).build())
        })
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
        let mut list = List::virtualized(50, 24.0, |index| {
            Box::new(Text::builder().content(index.to_string()).build())
        });
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
        let mut list = List::virtualized(30, 30.0, |index| {
            Box::new(Text::builder().content(index.to_string()).build())
        })
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
            Box::new(
                Container::column().child(Text::builder().content(format!("Row {index}")).build()),
            )
        });
        list.push_item(Text::builder().content("Extra").build());
        assert_eq!(list.len(), 4);
        assert!(matches!(list.mode, ListMode::Owned(_)));
    }

    #[test]
    fn configuration_methods_update_style() {
        let list = List::empty()
            .direction(ListDirection::Horizontal)
            .gap(10.0)
            .padding(12.0)
            .fill_width()
            .fill_height();

        assert_eq!(list.direction, ListDirection::Horizontal);
        assert_eq!(list.style.flex_direction, FlexDirection::Row);
        assert_eq!(list.style.gap.width, length(10.0));
        assert_eq!(list.style.padding.left, length(12.0));
        assert_eq!(list.style.size.width, percent(1.0));
        assert_eq!(list.style.size.height, percent(1.0));
    }

    #[test]
    fn virtualized_builder_matches_positional_constructor() {
        let built = List::virtualized_builder()
            .item_count(25)
            .item_extent(28.0)
            .item_builder(|index| Box::new(Text::builder().content(index.to_string()).build()))
            .direction(ListDirection::Horizontal)
            .overscan(4)
            .padding(10.0)
            .fill_width(true)
            .build();

        let direct = List::virtualized(25, 28.0, |index| {
            Box::new(Text::builder().content(index.to_string()).build())
        })
        .direction(ListDirection::Horizontal)
        .overscan(4)
        .padding(10.0)
        .fill_width();

        assert_eq!(built.direction, direct.direction);
        assert_eq!(built.style.padding, direct.style.padding);
        assert_eq!(built.style.size.width, direct.style.size.width);

        match (&built.mode, &direct.mode) {
            (ListMode::Virtualized(built_state), ListMode::Virtualized(direct_state)) => {
                assert_eq!(built_state.item_count, direct_state.item_count);
                assert_eq!(built_state.item_extent, direct_state.item_extent);
                assert_eq!(built_state.overscan, direct_state.overscan);
            }
            _ => panic!("expected virtualized lists"),
        }
    }
}
