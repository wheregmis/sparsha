//! Keyed collection helper for declarative child rendering.

use crate::{IntoWidget, Widget};
use sparsh_core::Color;
use sparsh_layout::WidgetId;
use taffy::prelude::*;

/// A keyed flex container for rendering dynamic collections declaratively.
pub struct ForEach {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
    child_keys: Vec<usize>,
    style: Style,
    background: Option<Color>,
    corner_radius: f32,
}

impl ForEach {
    /// Build a keyed collection widget from the provided items.
    pub fn new<T, I, K, KF, RF, W>(items: I, key_fn: KF, render: RF) -> Self
    where
        I: IntoIterator<Item = T>,
        KF: Fn(&T) -> K,
        K: Into<usize>,
        RF: Fn(T) -> W,
        W: IntoWidget,
    {
        let mut child_keys = Vec::new();
        let mut children = Vec::new();
        for item in items {
            child_keys.push(key_fn(&item).into());
            children.push(render(item).into_widget());
        }

        Self {
            id: WidgetId::default(),
            children,
            child_keys,
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            background: None,
            corner_radius: 0.0,
        }
    }

    pub fn direction(mut self, direction: FlexDirection) -> Self {
        self.style.flex_direction = direction;
        self
    }

    pub fn row(self) -> Self {
        self.direction(FlexDirection::Row)
    }

    pub fn column(self) -> Self {
        self.direction(FlexDirection::Column)
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = Size {
            width: length(gap),
            height: length(gap),
        };
        self
    }

    pub fn padding(mut self, all: f32) -> Self {
        self.style.padding = Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        self
    }

    pub fn fill_width(mut self) -> Self {
        self.style.size.width = percent(1.0);
        self
    }

    pub fn fill_height(mut self) -> Self {
        self.style.size.height = percent(1.0);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }
}

impl Widget for ForEach {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn paint(&self, ctx: &mut crate::PaintContext) {
        if let Some(background) = self.background {
            let bounds = ctx.bounds();
            if self.corner_radius > 0.0 {
                ctx.fill_rounded_rect(bounds, background, self.corner_radius);
            } else {
                ctx.fill_rect(bounds, background);
            }
        }
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }

    fn child_path_key(&self, child_position: usize) -> usize {
        self.child_keys
            .get(child_position)
            .copied()
            .unwrap_or(child_position)
    }

    fn child_slot_for_path_key(&self, key: usize) -> Option<usize> {
        self.child_keys
            .iter()
            .position(|candidate| *candidate == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Text;

    #[test]
    fn for_each_uses_stable_child_keys() {
        let list = ForEach::new(
            vec![(10usize, "A"), (20usize, "B"), (30usize, "C")],
            |item| item.0,
            |item| Text::new(item.1),
        );

        assert_eq!(list.child_path_key(0), 10);
        assert_eq!(list.child_path_key(1), 20);
        assert_eq!(list.child_slot_for_path_key(30), Some(2));
        assert_eq!(list.child_slot_for_path_key(99), None);
    }
}
