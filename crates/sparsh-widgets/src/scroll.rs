//! Scrollable container widget.

use crate::{EventContext, PaintContext, Widget};
use sparsh_core::{Color, Rect};
use sparsh_input::InputEvent;
use sparsh_layout::WidgetId;
use std::cell::Cell;
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
            thumb_color: Color::from_hex(0x9CA3AF),
            thumb_hover_color: Color::from_hex(0x6B7280),
            width: 8.0,
            corner_radius: 4.0,
        }
    }
}

/// A scrollable container widget.
pub struct Scroll {
    id: WidgetId,
    content: Option<Box<dyn Widget>>,
    direction: ScrollDirection,
    offset_x: f32,
    offset_y: f32,
    content_size: Cell<(f32, f32)>,
    style: ScrollbarStyle,
    layout_style: Style,
    dragging_scrollbar: bool,
    hover_scrollbar: bool,
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
            offset_x: 0.0,
            offset_y: 0.0,
            content_size: Cell::new((0.0, 0.0)),
            style: ScrollbarStyle::default(),
            layout_style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                overflow: Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                ..Default::default()
            },
            dragging_scrollbar: false,
            hover_scrollbar: false,
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
        self.style = style;
        self
    }

    /// Enable or disable debug overlay.
    pub fn debug_overlay(mut self, enabled: bool) -> Self {
        self.debug_overlay = enabled;
        self
    }

    /// Get the current scroll offset.
    pub fn offset(&self) -> (f32, f32) {
        (self.offset_x, self.offset_y)
    }

    /// Set the scroll offset.
    pub fn set_offset(&mut self, x: f32, y: f32) {
        self.offset_x = x.max(0.0);
        self.offset_y = y.max(0.0);
    }

    /// Scroll to ensure a rectangle is visible.
    pub fn scroll_to_visible(&mut self, rect: Rect, viewport: Rect) {
        // Vertical
        if rect.y < self.offset_y {
            self.offset_y = rect.y;
        } else if rect.y + rect.height > self.offset_y + viewport.height {
            self.offset_y = rect.y + rect.height - viewport.height;
        }

        // Horizontal
        if rect.x < self.offset_x {
            self.offset_x = rect.x;
        } else if rect.x + rect.width > self.offset_x + viewport.width {
            self.offset_x = rect.x + rect.width - viewport.width;
        }
    }

    fn clamp_offset(&mut self, viewport: Rect) {
        let content_size = self.content_size.get();
        let max_x = (content_size.0 - viewport.width).max(0.0);
        let max_y = (content_size.1 - viewport.height).max(0.0);
        self.offset_x = self.offset_x.clamp(0.0, max_x);
        self.offset_y = self.offset_y.clamp(0.0, max_y);
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

    fn update_content_size(&mut self, layout_tree: &sparsh_layout::LayoutTree) {
        let size = self.content_size_from_tree(layout_tree);
        self.content_size.set(size);
    }

    fn scrollbar_rect(&self, viewport: Rect) -> Option<Rect> {
        self.scrollbar_rect_for(viewport, self.content_size.get())
    }

    fn scrollbar_rect_for(&self, viewport: Rect, content_size: (f32, f32)) -> Option<Rect> {
        match self.direction {
            ScrollDirection::Vertical | ScrollDirection::Both => {
                if content_size.1 <= viewport.height {
                    return None;
                }

                let track_height = viewport.height;
                let thumb_height = (viewport.height / content_size.1 * track_height).max(20.0);
                let thumb_y = (self.offset_y / (content_size.1 - viewport.height))
                    * (track_height - thumb_height);

                Some(Rect::new(
                    viewport.x + viewport.width - self.style.width,
                    viewport.y + thumb_y,
                    self.style.width,
                    thumb_height,
                ))
            }
            ScrollDirection::Horizontal => {
                if content_size.0 <= viewport.width {
                    return None;
                }

                let track_width = viewport.width;
                let thumb_width = (viewport.width / content_size.0 * track_width).max(20.0);
                let thumb_x = (self.offset_x / (content_size.0 - viewport.width))
                    * (track_width - thumb_width);

                Some(Rect::new(
                    viewport.x + thumb_x,
                    viewport.y + viewport.height - self.style.width,
                    thumb_width,
                    self.style.width,
                ))
            }
        }
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

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();

        self.content_size
            .set(self.content_size_from_tree(ctx.layout_tree));

        // Clip content
        ctx.push_clip(bounds);

        // Translate content by negative scroll offset (physical pixels)
        let offset_x_physical = -self.offset_x * ctx.scale_factor;
        let offset_y_physical = -self.offset_y * ctx.scale_factor;
        ctx.push_translation((offset_x_physical, offset_y_physical));
    }

    fn paint_after_children(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let scale_factor = ctx.scale_factor;

        // Pop translation and clip in reverse order
        ctx.pop_translation();
        ctx.pop_clip();

        // Draw scrollbar on top (use logical bounds, then scale to physical)
        let logical_bounds = Rect::new(
            bounds.x / scale_factor,
            bounds.y / scale_factor,
            bounds.width / scale_factor,
            bounds.height / scale_factor,
        );

        let content_size = self.content_size_from_tree(ctx.layout_tree);
        if let Some(scrollbar) = self.scrollbar_rect_for(logical_bounds, content_size) {
            let track_logical = match self.direction {
                ScrollDirection::Vertical | ScrollDirection::Both => Rect::new(
                    logical_bounds.x + logical_bounds.width - self.style.width,
                    logical_bounds.y,
                    self.style.width,
                    logical_bounds.height,
                ),
                ScrollDirection::Horizontal => Rect::new(
                    logical_bounds.x,
                    logical_bounds.y + logical_bounds.height - self.style.width,
                    logical_bounds.width,
                    self.style.width,
                ),
            };

            let track = Rect::new(
                track_logical.x * scale_factor,
                track_logical.y * scale_factor,
                track_logical.width * scale_factor,
                track_logical.height * scale_factor,
            );
            ctx.fill_rounded_rect(track, self.style.track_color, self.style.corner_radius);

            let scrollbar = Rect::new(
                scrollbar.x * scale_factor,
                scrollbar.y * scale_factor,
                scrollbar.width * scale_factor,
                scrollbar.height * scale_factor,
            );

            let thumb_color = if self.hover_scrollbar || self.dragging_scrollbar {
                self.style.thumb_hover_color
            } else {
                self.style.thumb_color
            };
            ctx.fill_rounded_rect(scrollbar, thumb_color, self.style.corner_radius);
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

            let debug_text = format!(
                "viewport: {:.0}x{:.0}\ncontent: {:.0}x{:.0}\noffset: {:.0},{:.0}",
                logical_bounds.width,
                logical_bounds.height,
                content_size.0,
                content_size.1,
                self.offset_x,
                self.offset_y
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
        let bounds = ctx.bounds();

        self.update_content_size(ctx.layout_tree);

        match event {
            InputEvent::Scroll { delta, pos } if ctx.contains(*pos) => {
                match self.direction {
                    ScrollDirection::Vertical => {
                        self.offset_y -= delta.y * 20.0;
                    }
                    ScrollDirection::Horizontal => {
                        self.offset_x -= delta.x * 20.0;
                    }
                    ScrollDirection::Both => {
                        self.offset_x -= delta.x * 20.0;
                        self.offset_y -= delta.y * 20.0;
                    }
                }
                self.clamp_offset(bounds);
                ctx.stop_propagation();
                ctx.request_paint();
            }
            InputEvent::PointerMove { pos } => {
                if let Some(scrollbar) = self.scrollbar_rect(bounds) {
                    let was_hover = self.hover_scrollbar;
                    self.hover_scrollbar = scrollbar.contains(*pos);
                    if was_hover != self.hover_scrollbar {
                        ctx.request_paint();
                    }
                }
            }
            _ => {}
        }
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        match &self.content {
            Some(c) => std::slice::from_ref(c),
            None => &[],
        }
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        match &mut self.content {
            Some(c) => std::slice::from_mut(c),
            None => &mut [],
        }
    }
}
