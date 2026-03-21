//! Scrollable container widget.

use crate::{
    current_theme,
    scroll_model::{ScrollAxes, ScrollModel, Scrollbars},
    AccessibilityAction, AccessibilityInfo, AccessibilityRole, EventContext, PaintContext, Widget,
};
use sparsh_core::{Color, Rect};
use sparsh_input::{InputEvent, Modifiers};
use sparsh_layout::WidgetId;
use taffy::prelude::*;
use taffy::{Overflow, Point};

/// Scroll direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Scroll vertically only.
    #[default]
    Vertical,
    /// Scroll horizontally only.
    Horizontal,
    /// Scroll in both directions.
    Both,
}

/// Style for scrollbar.
#[derive(Clone, Debug)]
pub struct ScrollbarStyle {
    pub track_color: Color,
    pub thumb_color: Color,
    pub thumb_hover_color: Color,
    pub width: f32,
    pub corner_radius: f32,
}

impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            track_color: Color::from_hex(0xE5E7EB),
            thumb_color: Color::from_hex(0x94A3B8),
            thumb_hover_color: Color::from_hex(0x64748B),
            width: 10.0,
            corner_radius: 4.0,
        }
    }
}

/// A scrollable container widget.
pub struct Scroll {
    id: WidgetId,
    content: Option<Box<dyn Widget>>,
    direction: ScrollDirection,
    model: ScrollModel,
    content_size: std::cell::Cell<(f32, f32)>,
    viewport_size: std::cell::Cell<(f32, f32)>,
    scrollbar_style_override: Option<ScrollbarStyle>,
    layout_style: Style,
    debug_overlay: bool,
}

impl Default for Scroll {
    fn default() -> Self {
        Self::new()
    }
}

impl Scroll {
    /// Create a new scroll container.
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            content: None,
            direction: ScrollDirection::Vertical,
            model: ScrollModel::default(),
            content_size: std::cell::Cell::new((0.0, 0.0)),
            viewport_size: std::cell::Cell::new((0.0, 0.0)),
            scrollbar_style_override: None,
            layout_style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                overflow: Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                ..Default::default()
            },
            debug_overlay: false,
        }
    }

    /// Set width.
    pub fn width(mut self, width: f32) -> Self {
        self.layout_style.size.width = length(width);
        self
    }

    /// Set height.
    pub fn height(mut self, height: f32) -> Self {
        self.layout_style.size.height = length(height);
        self
    }

    /// Set fixed size.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.layout_style.size = Size {
            width: length(width),
            height: length(height),
        };
        self
    }

    /// Fill available space.
    pub fn fill(mut self) -> Self {
        self.layout_style.size = Size {
            width: percent(1.0),
            height: percent(1.0),
        };
        self.layout_style.align_self = Some(AlignSelf::Stretch);
        self
    }

    /// Fill width.
    pub fn fill_width(mut self) -> Self {
        self.layout_style.size.width = percent(1.0);
        self
    }

    /// Fill height.
    pub fn fill_height(mut self) -> Self {
        self.layout_style.size.height = percent(1.0);
        self
    }

    /// Set flex grow.
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.layout_style.flex_grow = grow;
        self
    }

    /// Set flex shrink.
    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.layout_style.flex_shrink = shrink;
        self
    }

    /// Set the content widget.
    pub fn content(mut self, widget: impl Widget + 'static) -> Self {
        self.content = Some(Box::new(widget));
        self
    }

    /// Set the scroll direction.
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set to vertical scrolling only.
    pub fn vertical(mut self) -> Self {
        self.direction = ScrollDirection::Vertical;
        self
    }

    /// Set to horizontal scrolling only.
    pub fn horizontal(mut self) -> Self {
        self.direction = ScrollDirection::Horizontal;
        self
    }

    /// Set the scrollbar style.
    pub fn scrollbar_style(mut self, style: ScrollbarStyle) -> Self {
        self.scrollbar_style_override = Some(style);
        self
    }

    /// Enable or disable debug overlay.
    pub fn debug_overlay(mut self, enabled: bool) -> Self {
        self.debug_overlay = enabled;
        self
    }

    /// Get the current scroll offset.
    pub fn offset(&self) -> (f32, f32) {
        self.model.offset()
    }

    /// Set the scroll offset.
    pub fn set_offset(&mut self, x: f32, y: f32) {
        self.model.set_offset(x, y);
    }

    /// Scroll to ensure a rectangle is visible.
    pub fn scroll_to_visible(&mut self, rect: Rect, viewport: Rect) {
        let (mut x, mut y) = self.model.offset();

        if self.axes().vertical {
            if rect.y < y {
                y = rect.y;
            } else if rect.y + rect.height > y + viewport.height {
                y = rect.y + rect.height - viewport.height;
            }
        }

        if self.axes().horizontal {
            if rect.x < x {
                x = rect.x;
            } else if rect.x + rect.width > x + viewport.width {
                x = rect.x + rect.width - viewport.width;
            }
        }

        self.model.set_offset(x, y);
        self.model
            .clamp(viewport, self.content_size.get(), self.axes());
    }

    fn axes(&self) -> ScrollAxes {
        match self.direction {
            ScrollDirection::Vertical => ScrollAxes::new(false, true),
            ScrollDirection::Horizontal => ScrollAxes::new(true, false),
            ScrollDirection::Both => ScrollAxes::new(true, true),
        }
    }

    fn resolved_scrollbar_style(&self) -> ScrollbarStyle {
        self.scrollbar_style_override.clone().unwrap_or_else(|| {
            let theme = current_theme();
            ScrollbarStyle {
                track_color: theme.colors.surface_variant,
                thumb_color: theme.colors.border,
                thumb_hover_color: theme.colors.primary_hovered,
                width: theme.controls.scrollbar_thickness,
                corner_radius: theme.radii.md,
            }
        })
    }

    fn content_size_from_tree(&self, layout_tree: &sparsh_layout::LayoutTree) -> (f32, f32) {
        if let Some(content) = &self.content {
            if let Some(content_layout) = layout_tree.get_absolute_layout(content.id()) {
                let mut min_x = f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_y = f32::NEG_INFINITY;
                let mut found = false;

                fn visit(
                    widget: &dyn Widget,
                    layout_tree: &sparsh_layout::LayoutTree,
                    min_x: &mut f32,
                    min_y: &mut f32,
                    max_x: &mut f32,
                    max_y: &mut f32,
                    found: &mut bool,
                ) {
                    if let Some(layout) = layout_tree.get_absolute_layout(widget.id()) {
                        *found = true;
                        *min_x = min_x.min(layout.bounds.x);
                        *min_y = min_y.min(layout.bounds.y);
                        *max_x = max_x.max(layout.bounds.x + layout.bounds.width);
                        *max_y = max_y.max(layout.bounds.y + layout.bounds.height);
                    }

                    for child in widget.children() {
                        visit(
                            child.as_ref(),
                            layout_tree,
                            min_x,
                            min_y,
                            max_x,
                            max_y,
                            found,
                        );
                    }
                }

                visit(
                    content.as_ref(),
                    layout_tree,
                    &mut min_x,
                    &mut min_y,
                    &mut max_x,
                    &mut max_y,
                    &mut found,
                );

                if found {
                    let width = (max_x - content_layout.bounds.x).max(content_layout.bounds.width);
                    let height =
                        (max_y - content_layout.bounds.y).max(content_layout.bounds.height);
                    return (width, height);
                }
            }
        }
        self.content_size.get()
    }

    fn update_content_metrics(&mut self, viewport: Rect, layout_tree: &sparsh_layout::LayoutTree) {
        let content_size = self.content_size_from_tree(layout_tree);
        self.content_size.set(content_size);
        self.model.clamp(viewport, content_size, self.axes());
    }

    fn scrollbars(&self, viewport: Rect) -> Scrollbars {
        self.model.scrollbars(
            viewport,
            self.content_size.get(),
            self.resolved_scrollbar_style().width,
            self.axes(),
        )
    }

    fn accessibility_scroll_info(&self) -> AccessibilityInfo {
        let mut info = AccessibilityInfo::new(AccessibilityRole::ScrollView);
        match self.direction {
            ScrollDirection::Vertical => {
                info = info
                    .action(AccessibilityAction::ScrollUp)
                    .action(AccessibilityAction::ScrollDown);
            }
            ScrollDirection::Horizontal => {
                info = info
                    .action(AccessibilityAction::ScrollLeft)
                    .action(AccessibilityAction::ScrollRight);
            }
            ScrollDirection::Both => {
                info = info
                    .action(AccessibilityAction::ScrollUp)
                    .action(AccessibilityAction::ScrollDown)
                    .action(AccessibilityAction::ScrollLeft)
                    .action(AccessibilityAction::ScrollRight);
            }
        }

        let (viewport_width, viewport_height) = self.viewport_size.get();
        let (content_width, content_height) = self.content_size.get();
        let (offset_x, offset_y) = self.model.offset();
        let max_x = (content_width - viewport_width).max(0.0);
        let max_y = (content_height - viewport_height).max(0.0);
        let scroll_value = match self.direction {
            ScrollDirection::Vertical => {
                let percent = if max_y > 0.0 {
                    (offset_y / max_y * 100.0).round()
                } else {
                    0.0
                };
                format!("{percent:.0}%")
            }
            ScrollDirection::Horizontal => {
                let percent = if max_x > 0.0 {
                    (offset_x / max_x * 100.0).round()
                } else {
                    0.0
                };
                format!("{percent:.0}%")
            }
            ScrollDirection::Both => {
                let x_percent = if max_x > 0.0 {
                    (offset_x / max_x * 100.0).round()
                } else {
                    0.0
                };
                let y_percent = if max_y > 0.0 {
                    (offset_y / max_y * 100.0).round()
                } else {
                    0.0
                };
                format!("x {x_percent:.0}%, y {y_percent:.0}%")
            }
        };
        info.value(scroll_value)
    }
}

impl Widget for Scroll {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        self.layout_style.clone()
    }

    fn is_scroll_container(&self) -> bool {
        true
    }

    fn child_event_offset(&self) -> glam::Vec2 {
        let (offset_x, offset_y) = self.model.offset();
        glam::vec2(offset_x, offset_y)
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let scale_factor = ctx.scale_factor.max(1.0);
        self.viewport_size
            .set((bounds.width / scale_factor, bounds.height / scale_factor));
        self.content_size
            .set(self.content_size_from_tree(ctx.layout_tree));

        ctx.push_clip(bounds);
        let (offset_x, offset_y) = self.model.offset();
        ctx.push_translation((-offset_x * scale_factor, -offset_y * scale_factor));
    }

    fn paint_after_children(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let scale_factor = ctx.scale_factor.max(1.0);
        let style = self.resolved_scrollbar_style();

        ctx.pop_translation();
        ctx.pop_clip();

        let logical_bounds = Rect::new(
            bounds.x / scale_factor,
            bounds.y / scale_factor,
            bounds.width / scale_factor,
            bounds.height / scale_factor,
        );
        let scrollbars = self.scrollbars(logical_bounds);

        if let Some(track) = scrollbars.vertical_track {
            ctx.fill_rounded_rect(
                scale_rect(track, scale_factor),
                style.track_color,
                style.corner_radius,
            );
        }
        if let Some(track) = scrollbars.horizontal_track {
            ctx.fill_rounded_rect(
                scale_rect(track, scale_factor),
                style.track_color,
                style.corner_radius,
            );
        }
        if let Some(corner) = scrollbars.corner {
            ctx.fill_rect(scale_rect(corner, scale_factor), style.track_color);
        }

        let thumb_color = match self.model.hover_axis().or(self.model.dragging_axis()) {
            Some(_) => style.thumb_hover_color,
            None => style.thumb_color,
        };
        if let Some(thumb) = scrollbars.vertical_thumb {
            ctx.fill_rounded_rect(
                scale_rect(thumb, scale_factor),
                thumb_color,
                style.corner_radius,
            );
        }
        if let Some(thumb) = scrollbars.horizontal_thumb {
            ctx.fill_rounded_rect(
                scale_rect(thumb, scale_factor),
                thumb_color,
                style.corner_radius,
            );
        }

        if self.debug_overlay {
            use sparsh_text::TextStyle;

            let debug_bg = Rect::new(
                bounds.x + 8.0,
                bounds.y + 8.0,
                260.0 * scale_factor,
                54.0 * scale_factor,
            );
            ctx.fill_rounded_rect(debug_bg, Color::from_hex(0x0F172A).with_alpha(0.7), 6.0);

            let text_style = TextStyle::default()
                .with_size(11.0)
                .with_color(Color::from_hex(0xE2E8F0));
            let (offset_x, offset_y) = self.model.offset();
            let content_size = self.content_size.get();
            let debug_text = format!(
                "viewport: {:.0}x{:.0}\ncontent: {:.0}x{:.0}\noffset: {:.0},{:.0}",
                logical_bounds.width,
                logical_bounds.height,
                content_size.0,
                content_size.1,
                offset_x,
                offset_y
            );
            ctx.draw_text(
                &debug_text,
                &text_style,
                debug_bg.x + 8.0 * scale_factor,
                debug_bg.y + 6.0 * scale_factor,
            );
        }
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        let viewport = ctx.bounds();
        self.update_content_metrics(viewport, ctx.layout_tree);
        let scrollbar_width = self.resolved_scrollbar_style().width;

        match event {
            InputEvent::Scroll {
                delta,
                pos,
                modifiers,
            } if ctx.contains(*pos) => {
                if self.model.scroll_by(
                    viewport,
                    self.content_size.get(),
                    self.axes(),
                    *delta,
                    *modifiers,
                ) {
                    ctx.stop_propagation();
                    ctx.request_paint();
                }
            }
            InputEvent::PointerDown { pos, .. } if ctx.contains(*pos) => {
                let result = self.model.pointer_down(
                    *pos,
                    viewport,
                    self.content_size.get(),
                    scrollbar_width,
                    self.axes(),
                );
                if result.capture_pointer {
                    ctx.capture_pointer();
                }
                if result.consume {
                    ctx.stop_propagation();
                }
                if result.changed_offset || result.changed_visuals {
                    ctx.request_paint();
                }
            }
            InputEvent::PointerMove { pos } => {
                let result = self.model.pointer_move(
                    *pos,
                    viewport,
                    self.content_size.get(),
                    scrollbar_width,
                    self.axes(),
                );
                if result.consume {
                    ctx.stop_propagation();
                }
                if result.changed_offset || result.changed_visuals {
                    ctx.request_paint();
                }
            }
            InputEvent::PointerUp { pos, .. } => {
                let result = self.model.pointer_up(*pos);
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
        match &self.content {
            Some(content) => std::slice::from_ref(content),
            None => &[],
        }
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        match &mut self.content {
            Some(content) => std::slice::from_mut(content),
            None => &mut [],
        }
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        Some(self.accessibility_scroll_info())
    }

    fn handle_accessibility_action(
        &mut self,
        action: AccessibilityAction,
        _value: Option<String>,
    ) -> bool {
        let (viewport_width, viewport_height) = self.viewport_size.get();
        let viewport = Rect::new(0.0, 0.0, viewport_width.max(0.0), viewport_height.max(0.0));
        let delta = match action {
            AccessibilityAction::ScrollUp => glam::vec2(0.0, 0.8),
            AccessibilityAction::ScrollDown => glam::vec2(0.0, -0.8),
            AccessibilityAction::ScrollLeft => glam::vec2(0.8, 0.0),
            AccessibilityAction::ScrollRight => glam::vec2(-0.8, 0.0),
            _ => return false,
        };
        self.model.scroll_by(
            viewport,
            self.content_size.get(),
            self.axes(),
            delta,
            Modifiers::default(),
        )
    }
}

fn scale_rect(rect: Rect, scale_factor: f32) -> Rect {
    Rect::new(
        rect.x * scale_factor,
        rect.y * scale_factor,
        rect.width * scale_factor,
        rect.height * scale_factor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scroll_model::ScrollAxis;
    use crate::test_helpers::{
        layout_bounds, mock_event_context, pointer_down_at, pointer_move_at, pointer_up_at,
    };
    use crate::{Container, Text};
    use sparsh_input::FocusManager;
    use sparsh_layout::LayoutTree;

    #[test]
    fn both_axis_scroll_uses_wheel_delta_and_shift_fallback() {
        let viewport = Rect::new(0.0, 0.0, 100.0, 80.0);
        let mut model = ScrollModel::default();
        let axes = ScrollAxes::new(true, true);
        let content = (260.0, 260.0);

        let changed = model.scroll_by(
            viewport,
            content,
            axes,
            glam::vec2(1.0, -2.0),
            Modifiers::default(),
        );
        assert!(changed);
        assert_eq!(model.offset(), (0.0, 40.0));

        let changed = model.scroll_by(
            viewport,
            content,
            axes,
            glam::vec2(0.0, -2.0),
            Modifiers::SHIFT,
        );
        assert!(changed);
        assert_eq!(model.offset(), (40.0, 40.0));
    }

    #[test]
    fn both_axis_scrollbars_include_corner_gutter() {
        let mut model = ScrollModel::default();
        model.set_offset(20.0, 30.0);
        let bars = model.scrollbars(
            Rect::new(0.0, 0.0, 120.0, 90.0),
            (300.0, 240.0),
            10.0,
            ScrollAxes::new(true, true),
        );
        assert!(bars.horizontal_track.is_some());
        assert!(bars.vertical_track.is_some());
        assert!(bars.corner.is_some());
    }

    #[test]
    fn scrollbar_thumb_drag_updates_offset() {
        let viewport = Rect::new(0.0, 0.0, 120.0, 90.0);
        let content = (120.0, 360.0);
        let mut model = ScrollModel::default();
        let bars = model.scrollbars(viewport, content, 10.0, ScrollAxes::new(false, true));
        let thumb = bars.vertical_thumb.expect("vertical thumb");
        let center = glam::vec2(thumb.x + thumb.width * 0.5, thumb.y + thumb.height * 0.5);

        let down = model.pointer_down(
            center,
            viewport,
            content,
            10.0,
            ScrollAxes::new(false, true),
        );
        assert!(down.capture_pointer);

        let move_result = model.pointer_move(
            center + glam::vec2(0.0, 24.0),
            viewport,
            content,
            10.0,
            ScrollAxes::new(false, true),
        );
        assert!(move_result.changed_offset);
        assert!(model.offset().1 > 0.0);
    }

    #[test]
    fn scroll_widget_track_click_pages_content() {
        let mut scroll = Scroll::new().vertical().content(
            Container::new()
                .column()
                .child(Container::new().height(320.0).child(Text::new("Large"))),
        );
        scroll.set_id(Default::default());

        let layout_tree = LayoutTree::new();
        let layout = layout_bounds(0.0, 0.0, 120.0, 90.0);
        scroll.viewport_size.set((120.0, 90.0));
        scroll.content_size.set((120.0, 320.0));
        let mut focus = FocusManager::new();
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, scroll.id(), false);

        scroll.event(&mut ctx, &pointer_down_at(116.0, 72.0));
        assert!(scroll.offset().1 > 0.0);
    }

    #[test]
    fn hover_and_release_update_scrollbar_visual_state() {
        let mut scroll = Scroll::new();
        scroll.viewport_size.set((120.0, 90.0));
        scroll.content_size.set((120.0, 320.0));

        let layout_tree = LayoutTree::new();
        let layout = layout_bounds(0.0, 0.0, 120.0, 90.0);
        let mut focus = FocusManager::new();
        scroll.set_id(Default::default());
        let mut ctx = mock_event_context(layout, &layout_tree, &mut focus, scroll.id(), false);

        scroll.event(&mut ctx, &pointer_move_at(116.0, 24.0));
        assert_eq!(scroll.model.hover_axis(), Some(ScrollAxis::Vertical));

        ctx.has_capture = true;
        scroll.event(&mut ctx, &pointer_up_at(116.0, 24.0));
        assert!(ctx.commands.release_pointer || scroll.model.hover_axis().is_some());
    }
}
