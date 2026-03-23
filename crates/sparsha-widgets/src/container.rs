//! Container widget for laying out children.

use crate::{EventContext, IntoWidget, PaintContext, Widget};
use sparsha_core::Color;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
use taffy::prelude::*;

/// A container widget that lays out children using flexbox.
pub struct Container {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
    style: Style,
    background: Option<Color>,
    corner_radius: f32,
    border_width: f32,
    border_color: Color,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    /// Create a new empty container.
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            children: Vec::new(),
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background: None,
            corner_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        }
    }

    /// Add a child widget.
    pub fn child(mut self, widget: impl IntoWidget) -> Self {
        self.children.push(widget.into_widget());
        self
    }

    /// Add multiple child widgets.
    pub fn children<I, W>(mut self, widgets: I) -> Self
    where
        I: IntoIterator<Item = W>,
        W: IntoWidget,
    {
        self.children
            .extend(widgets.into_iter().map(IntoWidget::into_widget));
        self
    }

    /// Set the flex direction.
    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.style.flex_direction = direction;
        self
    }

    /// Make this a row container.
    pub fn row(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    /// Make this a column container.
    pub fn column(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    /// Set the gap between children.
    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = Size {
            width: length(gap),
            height: length(gap),
        };
        self
    }

    /// Set padding.
    pub fn padding(mut self, all: f32) -> Self {
        self.style.padding = Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        self
    }

    /// Set padding for each side.
    pub fn padding_sides(mut self, left: f32, right: f32, top: f32, bottom: f32) -> Self {
        self.style.padding = Rect {
            left: length(left),
            right: length(right),
            top: length(top),
            bottom: length(bottom),
        };
        self
    }

    /// Set the background color.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set corner radius.
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set border.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self
    }

    /// Set alignment.
    pub fn align_items(mut self, align: AlignItems) -> Self {
        self.style.align_items = Some(align);
        self
    }

    /// Set justify content.
    pub fn justify_content(mut self, justify: JustifyContent) -> Self {
        self.style.justify_content = Some(justify);
        self
    }

    /// Center children both horizontally and vertically.
    pub fn center(mut self) -> Self {
        self.style.align_items = Some(AlignItems::Center);
        self.style.justify_content = Some(JustifyContent::Center);
        self
    }

    /// Align children at the start (left for row, top for column).
    pub fn align_start(mut self) -> Self {
        self.style.align_items = Some(AlignItems::FlexStart);
        self.style.justify_content = Some(JustifyContent::FlexStart);
        self
    }

    /// Stretch children to fill the cross axis.
    pub fn stretch(mut self) -> Self {
        self.style.align_items = Some(AlignItems::Stretch);
        self
    }

    /// Space children evenly with space between them.
    pub fn space_between(mut self) -> Self {
        self.style.justify_content = Some(JustifyContent::SpaceBetween);
        self
    }

    /// Space children evenly with equal space around them.
    pub fn space_around(mut self) -> Self {
        self.style.justify_content = Some(JustifyContent::SpaceAround);
        self
    }

    /// Space children evenly with equal space between and around them.
    pub fn space_evenly(mut self) -> Self {
        self.style.justify_content = Some(JustifyContent::SpaceEvenly);
        self
    }

    /// Set fixed size.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.style.size = Size {
            width: length(width),
            height: length(height),
        };
        self
    }

    /// Set minimum size.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.style.min_size = Size {
            width: length(width),
            height: length(height),
        };
        self
    }

    /// Set width only (height auto).
    pub fn width(mut self, width: f32) -> Self {
        self.style.size.width = length(width);
        self
    }

    /// Set height only (width auto).
    pub fn height(mut self, height: f32) -> Self {
        self.style.size.height = length(height);
        self
    }

    /// Fill available space.
    pub fn fill(mut self) -> Self {
        self.style.size = Size {
            width: percent(1.0),
            height: percent(1.0),
        };
        self
    }

    /// Fill width only (height auto).
    pub fn fill_width(mut self) -> Self {
        self.style.size.width = percent(1.0);
        self
    }

    /// Fill height only (width auto).
    pub fn fill_height(mut self) -> Self {
        self.style.size.height = percent(1.0);
        self
    }

    /// Set flex grow.
    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self
    }

    /// Set flex shrink.
    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.style.flex_shrink = shrink;
        self
    }

    /// Enable flex wrapping.
    pub fn wrap(mut self) -> Self {
        self.style.flex_wrap = taffy::FlexWrap::Wrap;
        self
    }
}

impl Widget for Container {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();

        // Draw background
        if let Some(bg) = self.background {
            if self.border_width > 0.0 {
                ctx.fill_bordered_rect(
                    bounds,
                    bg,
                    self.corner_radius,
                    self.border_width,
                    self.border_color,
                );
            } else if self.corner_radius > 0.0 {
                ctx.fill_rounded_rect(bounds, bg, self.corner_radius);
            } else {
                ctx.fill_rect(bounds, bg);
            }
        }

        // Note: Children are painted by the framework traversal
    }

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_methods_update_style_and_visuals() {
        let mut container = Container::new()
            .direction(FlexDirection::Row)
            .gap(12.0)
            .padding(16.0)
            .background(Color::from_hex(0x0F172A))
            .corner_radius(10.0)
            .border(1.0, Color::from_hex(0xCBD5E1))
            .align_items(AlignItems::Center)
            .justify_content(JustifyContent::SpaceBetween)
            .width(320.0)
            .height(180.0)
            .flex_grow(1.0)
            .flex_shrink(0.0)
            .wrap();
        container.style.min_size.width = length(200.0);
        container.style.min_size.height = length(120.0);

        assert_eq!(container.style.flex_direction, FlexDirection::Row);
        assert_eq!(container.style.gap.width, length(12.0));
        assert_eq!(container.style.padding.left, length(16.0));
        assert_eq!(container.style.size.width, length(320.0));
        assert_eq!(container.style.size.height, length(180.0));
        assert_eq!(container.style.min_size.width, length(200.0));
        assert_eq!(container.style.min_size.height, length(120.0));
        assert_eq!(container.style.flex_grow, 1.0);
        assert_eq!(container.style.flex_shrink, 0.0);
        assert_eq!(container.style.flex_wrap, taffy::FlexWrap::Wrap);
        assert_eq!(container.background, Some(Color::from_hex(0x0F172A)));
        assert_eq!(container.corner_radius, 10.0);
        assert_eq!(container.border_width, 1.0);
        assert_eq!(container.border_color, Color::from_hex(0xCBD5E1));
    }
}
