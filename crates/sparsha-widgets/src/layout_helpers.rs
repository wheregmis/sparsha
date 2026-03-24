//! Semantic layout helpers.

use crate::{IntoWidget, PaintContext, Widget};
use sparsha_layout::WidgetId;
use taffy::prelude::*;

/// Alignment of a single child inside a full-size container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    TopStart,
    TopCenter,
    TopEnd,
    CenterStart,
    #[default]
    Center,
    CenterEnd,
    BottomStart,
    BottomCenter,
    BottomEnd,
}

impl Alignment {
    fn into_main_axis(self) -> JustifyContent {
        match self {
            Self::TopStart | Self::TopCenter | Self::TopEnd => JustifyContent::FlexStart,
            Self::CenterStart | Self::Center | Self::CenterEnd => JustifyContent::Center,
            Self::BottomStart | Self::BottomCenter | Self::BottomEnd => JustifyContent::FlexEnd,
        }
    }

    fn into_cross_axis(self) -> AlignItems {
        match self {
            Self::TopStart | Self::CenterStart | Self::BottomStart => AlignItems::FlexStart,
            Self::TopCenter | Self::Center | Self::BottomCenter => AlignItems::Center,
            Self::TopEnd | Self::CenterEnd | Self::BottomEnd => AlignItems::FlexEnd,
        }
    }
}

/// A zero-sized flexible gap that expands along the parent flex axis.
pub struct Spacer {
    id: WidgetId,
    flex: f32,
}

impl Spacer {
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            flex: 1.0,
        }
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex.max(0.0);
        self
    }
}

impl Widget for Spacer {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            flex_grow: self.flex,
            flex_shrink: 1.0,
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}
}

/// A semantic flex child that expands along the parent's main axis.
pub struct Expanded {
    id: WidgetId,
    child: Box<dyn Widget>,
    flex: f32,
}

impl Expanded {
    pub fn new(child: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            child: child.into_widget(),
            flex: 1.0,
        }
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex.max(0.0);
        self
    }
}

impl Widget for Expanded {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            flex_grow: self.flex,
            flex_shrink: 1.0,
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        self.child.measure(ctx)
    }
}

/// A semantic overlay container that layers children on top of each other.
pub struct Stack {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl IntoWidget) -> Self {
        self.children.push(Positioned::new(child).into_widget());
        self
    }

    pub fn aligned(mut self, alignment: Alignment, child: impl IntoWidget) -> Self {
        self.children
            .push(Positioned::fill(Align::new(child).alignment(alignment)).into_widget());
        self
    }

    pub fn positioned(mut self, child: Positioned) -> Self {
        self.children.push(child.into_widget());
        self
    }
}

impl Widget for Stack {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            position: Position::Relative,
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}

/// A semantic fixed-size box.
pub struct SizedBox {
    id: WidgetId,
    child: Option<Box<dyn Widget>>,
    width: Option<f32>,
    height: Option<f32>,
}

impl SizedBox {
    pub fn new() -> Self {
        Self {
            id: WidgetId::default(),
            child: None,
            width: None,
            height: None,
        }
    }

    pub fn child(mut self, child: impl IntoWidget) -> Self {
        self.child = Some(child.into_widget());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(0.0));
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height.max(0.0));
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width.max(0.0));
        self.height = Some(height.max(0.0));
        self
    }
}

impl Widget for SizedBox {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            size: Size {
                width: self.width.map(length).unwrap_or_else(auto),
                height: self.height.map(length).unwrap_or_else(auto),
            },
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        self.child.as_ref().map_or(&[], std::slice::from_ref)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        self.child.as_mut().map_or(&mut [], std::slice::from_mut)
    }
}

/// A semantic padding wrapper around a single child.
pub struct Padding {
    id: WidgetId,
    child: Box<dyn Widget>,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl Padding {
    pub fn all(amount: f32, child: impl IntoWidget) -> Self {
        Self::only(amount, amount, amount, amount, child)
    }

    pub fn symmetric(horizontal: f32, vertical: f32, child: impl IntoWidget) -> Self {
        Self::only(horizontal, horizontal, vertical, vertical, child)
    }

    pub fn horizontal(amount: f32, child: impl IntoWidget) -> Self {
        Self::symmetric(amount, 0.0, child)
    }

    pub fn vertical(amount: f32, child: impl IntoWidget) -> Self {
        Self::symmetric(0.0, amount, child)
    }

    pub fn only(left: f32, right: f32, top: f32, bottom: f32, child: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            child: child.into_widget(),
            left: left.max(0.0),
            right: right.max(0.0),
            top: top.max(0.0),
            bottom: bottom.max(0.0),
        }
    }
}

impl Widget for Padding {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            padding: Rect {
                left: length(self.left),
                right: length(self.right),
                top: length(self.top),
                bottom: length(self.bottom),
            },
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }
}

/// A semantic absolute-positioning wrapper for a single child inside a [`Stack`].
pub struct Positioned {
    id: WidgetId,
    child: Box<dyn Widget>,
    left: Option<f32>,
    right: Option<f32>,
    top: Option<f32>,
    bottom: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
}

impl Positioned {
    pub fn new(child: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            child: child.into_widget(),
            left: None,
            right: None,
            top: None,
            bottom: None,
            width: None,
            height: None,
        }
    }

    pub fn fill(child: impl IntoWidget) -> Self {
        Self::new(child).left(0.0).right(0.0).top(0.0).bottom(0.0)
    }

    pub fn left(mut self, value: f32) -> Self {
        self.left = Some(value.max(0.0));
        self
    }

    pub fn right(mut self, value: f32) -> Self {
        self.right = Some(value.max(0.0));
        self
    }

    pub fn top(mut self, value: f32) -> Self {
        self.top = Some(value.max(0.0));
        self
    }

    pub fn bottom(mut self, value: f32) -> Self {
        self.bottom = Some(value.max(0.0));
        self
    }

    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value.max(0.0));
        self
    }

    pub fn height(mut self, value: f32) -> Self {
        self.height = Some(value.max(0.0));
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width.max(0.0));
        self.height = Some(height.max(0.0));
        self
    }
}

impl Widget for Positioned {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            position: Position::Absolute,
            inset: Rect {
                left: self.left.map(length).unwrap_or_else(auto),
                right: self.right.map(length).unwrap_or_else(auto),
                top: self.top.map(length).unwrap_or_else(auto),
                bottom: self.bottom.map(length).unwrap_or_else(auto),
            },
            size: Size {
                width: self.width.map(length).unwrap_or_else(auto),
                height: self.height.map(length).unwrap_or_else(auto),
            },
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        self.child.measure(ctx)
    }
}

/// A semantic full-size alignment wrapper for a single child.
pub struct Align {
    id: WidgetId,
    child: Box<dyn Widget>,
    alignment: Alignment,
}

impl Align {
    pub fn new(child: impl IntoWidget) -> Self {
        Self::center(child)
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn top_start(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::TopStart)
    }

    pub fn top_center(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::TopCenter)
    }

    pub fn top_end(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::TopEnd)
    }

    pub fn center_start(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::CenterStart)
    }

    pub fn center(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::Center)
    }

    pub fn center_end(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::CenterEnd)
    }

    pub fn bottom_start(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::BottomStart)
    }

    pub fn bottom_center(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::BottomCenter)
    }

    pub fn bottom_end(child: impl IntoWidget) -> Self {
        Self::with_alignment(child, Alignment::BottomEnd)
    }

    fn with_alignment(child: impl IntoWidget, alignment: Alignment) -> Self {
        Self {
            id: WidgetId::default(),
            child: child.into_widget(),
            alignment,
        }
    }
}

impl Widget for Align {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            justify_content: Some(self.alignment.into_main_axis()),
            align_items: Some(self.alignment.into_cross_axis()),
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        self.child.measure(ctx)
    }
}

/// A semantic full-size centering wrapper for a single child.
pub struct Center {
    id: WidgetId,
    child: Box<dyn Widget>,
}

impl Center {
    pub fn new(child: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            child: child.into_widget(),
        }
    }
}

impl Widget for Center {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            justify_content: Some(JustifyContent::Center),
            align_items: Some(AlignItems::Center),
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn measure(&self, ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        self.child.measure(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Text, TextAlign};

    #[test]
    fn spacer_expands_along_the_parent_flex_axis() {
        let spacer = Spacer::new().flex(2.0);
        assert_eq!(spacer.style().flex_grow, 2.0);
        assert_eq!(spacer.style().flex_shrink, 1.0);
    }

    #[test]
    fn expanded_maps_to_flex_growth() {
        let expanded = Expanded::new(Text::builder().content("body").build()).flex(3.0);
        let style = expanded.style();
        assert_eq!(style.flex_grow, 3.0);
        assert_eq!(style.flex_shrink, 1.0);
    }

    #[test]
    fn stack_child_defaults_to_absolute_overlay_layer() {
        let stack = Stack::new().child(Text::builder().content("layer").build());
        assert_eq!(stack.children().len(), 1);
        assert_eq!(stack.children()[0].style().position, Position::Absolute);
    }

    #[test]
    fn positioned_maps_absolute_insets_and_size() {
        let positioned = Positioned::new(Text::builder().content("fab").build())
            .left(12.0)
            .right(16.0)
            .top(20.0)
            .bottom(24.0)
            .size(56.0, 56.0);
        let style = positioned.style();
        assert_eq!(style.position, Position::Absolute);
        assert_eq!(style.inset.left, length(12.0));
        assert_eq!(style.inset.right, length(16.0));
        assert_eq!(style.inset.top, length(20.0));
        assert_eq!(style.inset.bottom, length(24.0));
        assert_eq!(style.size.width, length(56.0));
        assert_eq!(style.size.height, length(56.0));
    }

    #[test]
    fn stack_aligned_wraps_child_in_fill_layer() {
        let stack = Stack::new().aligned(
            Alignment::BottomEnd,
            Text::builder().content("badge").build(),
        );
        let style = stack.children()[0].style();
        assert_eq!(style.position, Position::Absolute);
        assert_eq!(style.inset.left, length(0.0));
        assert_eq!(style.inset.right, length(0.0));
        assert_eq!(style.inset.top, length(0.0));
        assert_eq!(style.inset.bottom, length(0.0));
    }

    #[test]
    fn sized_box_tracks_explicit_dimensions() {
        let box_ = SizedBox::new().width(120.0).height(48.0);
        assert_eq!(box_.style().size.width, length(120.0));
        assert_eq!(box_.style().size.height, length(48.0));
    }

    #[test]
    fn padding_maps_edge_insets_to_style_padding() {
        let padding = Padding::only(
            12.0,
            16.0,
            20.0,
            24.0,
            Text::builder().content("padded").build(),
        );
        let style = padding.style();
        assert_eq!(style.padding.left, length(12.0));
        assert_eq!(style.padding.right, length(16.0));
        assert_eq!(style.padding.top, length(20.0));
        assert_eq!(style.padding.bottom, length(24.0));
    }

    #[test]
    fn align_maps_to_full_size_flex_alignment() {
        let align = Align::bottom_end(
            Text::builder()
                .content("label")
                .align(TextAlign::Center)
                .build(),
        );
        let style = align.style();
        assert_eq!(style.size.width, percent(1.0));
        assert_eq!(style.size.height, percent(1.0));
        assert_eq!(style.justify_content, Some(JustifyContent::FlexEnd));
        assert_eq!(style.align_items, Some(AlignItems::FlexEnd));
    }

    #[test]
    fn center_maps_to_full_size_centering() {
        let center = Center::new(Text::builder().content("centered").build());
        let style = center.style();
        assert_eq!(style.size.width, percent(1.0));
        assert_eq!(style.size.height, percent(1.0));
        assert_eq!(style.justify_content, Some(JustifyContent::Center));
        assert_eq!(style.align_items, Some(AlignItems::Center));
    }
}
