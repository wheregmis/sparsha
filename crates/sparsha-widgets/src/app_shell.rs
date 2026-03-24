//! Semantic application shell widgets.

use crate::{
    control_state::{focus_ring_border_width, focus_ring_bounds, focus_ring_color, ControlState},
    current_theme, responsive_typography, AccessibilityAction, AccessibilityInfo,
    AccessibilityRole, Align, ButtonState, ButtonStyle, EventContext, IntoWidget, Padding,
    PaintContext, Positioned, Widget,
};
use bon::bon;
use sparsha_core::Color;
use sparsha_input::InputEvent;
use sparsha_layout::WidgetId;
use sparsha_text::TextStyle;
use taffy::prelude::*;

/// Top application bar used by the starter-style shell.
pub struct AppBar {
    id: WidgetId,
    title: String,
    background: Option<Color>,
    foreground: Option<Color>,
    height: f32,
    center_title: bool,
    padding_h: f32,
}

impl AppBar {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: WidgetId::default(),
            title: title.into(),
            background: None,
            foreground: None,
            height: 56.0,
            center_title: false,
            padding_h: 20.0,
        }
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(0.0);
        self
    }

    pub fn center_title(mut self, center_title: bool) -> Self {
        self.center_title = center_title;
        self
    }

    pub fn padding_h(mut self, padding_h: f32) -> Self {
        self.padding_h = padding_h.max(0.0);
        self
    }

    fn resolved_background(&self) -> Color {
        self.background
            .unwrap_or_else(|| current_theme().colors.primary)
    }

    fn resolved_foreground(&self) -> Color {
        self.foreground.unwrap_or(Color::WHITE)
    }
}

impl Widget for AppBar {
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
                height: length(self.height),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let theme = current_theme();
        let background = self.resolved_background();
        let foreground = self.resolved_foreground();
        let typography = responsive_typography(&theme);
        let text_style = TextStyle::default()
            .with_family(theme.typography.font_family.clone())
            .with_size(typography.title_size)
            .with_color(foreground);

        ctx.fill_rect(bounds, background);

        if self.center_title {
            ctx.draw_text_centered(&self.title, &text_style, bounds);
        } else {
            ctx.draw_text_aligned(&self.title, &text_style, bounds, self.padding_h);
        }
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        Some(
            AccessibilityInfo::new(AccessibilityRole::Label)
                .label(self.title.clone())
                .hidden(false),
        )
    }

    fn measure(&self, _ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        Some((0.0, self.height))
    }
}

/// Material-style floating action button.
pub struct FloatingActionButton {
    id: WidgetId,
    label: String,
    style_override: Option<ButtonStyle>,
    background_override: Option<Color>,
    text_color_override: Option<Color>,
    corner_radius_override: Option<f32>,
    disabled: bool,
    interaction: ControlState,
    on_click: Option<Box<dyn FnMut()>>,
    accessibility_label: Option<String>,
}

impl FloatingActionButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: WidgetId::default(),
            label: label.into(),
            style_override: None,
            background_override: None,
            text_color_override: None,
            corner_radius_override: None,
            disabled: false,
            interaction: ControlState::default(),
            on_click: None,
            accessibility_label: None,
        }
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style_override = Some(style);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background_override = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color_override = Some(color);
        self
    }

    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius_override = Some(radius.max(0.0));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn on_click(mut self, handler: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn state(&self) -> ButtonState {
        if self.disabled {
            ButtonState::Disabled
        } else if self.interaction.pressed() {
            ButtonState::Pressed
        } else if self.interaction.hovered() {
            ButtonState::Hovered
        } else {
            ButtonState::Normal
        }
    }

    fn themed_default_style() -> ButtonStyle {
        let theme = current_theme();
        let typography = responsive_typography(&theme);
        ButtonStyle {
            background: theme.colors.primary,
            background_hovered: theme.colors.primary_hovered,
            background_pressed: theme.colors.primary_pressed,
            background_disabled: theme.colors.disabled,
            text_color: Color::WHITE,
            text_color_disabled: theme.colors.text_muted,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            corner_radius: 28.0,
            padding_h: 0.0,
            padding_v: 0.0,
            font_size: typography.button_size.max(20.0),
            min_width: 56.0,
            min_height: 56.0,
        }
    }

    fn resolved_style(&self) -> ButtonStyle {
        let mut style = self
            .style_override
            .clone()
            .unwrap_or_else(Self::themed_default_style);

        if let Some(background) = self.background_override {
            style.background = background;
        }
        if let Some(text_color) = self.text_color_override {
            style.text_color = text_color;
        }
        if let Some(corner_radius) = self.corner_radius_override {
            style.corner_radius = corner_radius;
        }

        if style.min_width <= 0.0 {
            style.min_width = 56.0;
        }
        if style.min_height <= 0.0 {
            style.min_height = 56.0;
        }

        style
    }

    fn current_background(&self, style: &ButtonStyle) -> Color {
        match self.state() {
            ButtonState::Normal => style.background,
            ButtonState::Hovered => style.background_hovered,
            ButtonState::Pressed => style.background_pressed,
            ButtonState::Disabled => style.background_disabled,
        }
    }

    fn current_text_color(&self, style: &ButtonStyle) -> Color {
        match self.state() {
            ButtonState::Disabled => style.text_color_disabled,
            _ => style.text_color,
        }
    }
}

#[bon]
impl FloatingActionButton {
    #[builder(
        start_fn(name = builder, vis = "pub"),
        finish_fn(name = build, vis = "pub"),
        builder_type(name = FloatingActionButtonBuilder, vis = "pub"),
        state_mod(vis = "pub")
    )]
    fn builder_init(
        #[builder(into)] label: String,
        style: Option<ButtonStyle>,
        background: Option<Color>,
        text_color: Option<Color>,
        corner_radius: Option<f32>,
        #[builder(default)] disabled: bool,
        #[builder(with = |handler: impl FnMut() + 'static| Box::new(handler) as Box<dyn FnMut()>)]
        on_click: Option<Box<dyn FnMut()>>,
        accessibility_label: Option<String>,
    ) -> Self {
        let mut fab = Self::new(label);
        fab.style_override = style;
        fab.background_override = background;
        fab.text_color_override = text_color;
        fab.corner_radius_override = corner_radius;
        fab.disabled = disabled;
        fab.on_click = on_click;
        fab.accessibility_label = accessibility_label;
        fab
    }
}

impl Widget for FloatingActionButton {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        let style = self.resolved_style();
        Style {
            min_size: Size {
                width: length(style.min_width),
                height: length(style.min_height),
            },
            align_items: Some(AlignItems::Center),
            justify_content: Some(JustifyContent::Center),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let style = self.resolved_style();
        let bounds = ctx.bounds();
        let bg = self.current_background(&style);
        let text_color = self.current_text_color(&style);
        let scale = ctx.scale_factor;

        if style.border_width > 0.0 {
            ctx.fill_bordered_rect(
                bounds,
                bg,
                style.corner_radius,
                style.border_width,
                style.border_color,
            );
        } else {
            ctx.fill_rounded_rect(bounds, bg, style.corner_radius);
        }

        if ctx.has_focus() && !self.disabled {
            let controls = current_theme().controls;
            let focus_bounds = focus_ring_bounds(bounds, scale, &controls);
            ctx.fill_bordered_rect(
                focus_bounds,
                Color::TRANSPARENT,
                style.corner_radius + 2.0,
                focus_ring_border_width(scale, &controls),
                focus_ring_color(current_theme().colors.border_focus),
            );
        }

        let text_style = TextStyle::default()
            .with_family(current_theme().typography.font_family.clone())
            .with_size(style.font_size)
            .with_color(text_color);
        ctx.draw_text_centered(&self.label, &text_style, bounds);
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        if self.disabled {
            return;
        }

        match event {
            InputEvent::PointerMove { pos }
                if self.interaction.pointer_move(ctx.contains(*pos)) =>
            {
                ctx.request_paint();
            }
            InputEvent::PointerDown { pos, .. }
                if self.interaction.pointer_down(ctx.contains(*pos)) =>
            {
                ctx.capture_pointer();
            }
            InputEvent::PointerUp { pos, .. } if self.interaction.pressed() => {
                let should_click = self.interaction.pointer_up(ctx.contains(*pos));
                ctx.release_pointer();
                if should_click {
                    if let Some(handler) = &mut self.on_click {
                        handler();
                    }
                }
            }
            InputEvent::KeyDown { .. } if ctx.has_focus() => {
                use sparsha_input::{ActionMapper, StandardAction};
                let mapper = ActionMapper::new();
                if mapper.is_action(event, StandardAction::Activate) {
                    if let Some(handler) = &mut self.on_click {
                        handler();
                    }
                    ctx.stop_propagation();
                    ctx.request_paint();
                }
            }
            _ => {}
        }
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }

    fn measure(&self, _ctx: &mut crate::LayoutContext) -> Option<(f32, f32)> {
        Some((56.0, 56.0))
    }

    fn accessibility_info(&self) -> Option<AccessibilityInfo> {
        let label = self
            .accessibility_label
            .clone()
            .unwrap_or_else(|| self.label.clone());

        Some(
            AccessibilityInfo::new(AccessibilityRole::Button)
                .label(label)
                .disabled(self.disabled)
                .action(AccessibilityAction::Focus)
                .action(AccessibilityAction::Click),
        )
    }

    fn handle_accessibility_action(
        &mut self,
        action: AccessibilityAction,
        _value: Option<String>,
    ) -> bool {
        if self.disabled {
            return false;
        }

        if matches!(action, AccessibilityAction::Click) {
            if let Some(handler) = &mut self.on_click {
                handler();
            }
            return true;
        }

        false
    }
}

/// A simple application scaffold for app bars, content, and floating actions.
pub struct Scaffold {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
    background: Option<Color>,
}

struct ScaffoldBody {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl ScaffoldBody {
    fn new(body: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            children: vec![Positioned::fill(body).into_widget()],
        }
    }

    fn with_fab(mut self, fab: FloatingActionButton) -> Self {
        self.children.truncate(1);
        self.children.push(
            Positioned::fill(Align::bottom_end(Padding::only(24.0, 24.0, 0.0, 28.0, fab)))
                .into_widget(),
        );
        self
    }
}

impl Widget for ScaffoldBody {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            size: Size {
                width: percent(1.0),
                height: auto(),
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

impl Scaffold {
    pub fn new(body: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            children: vec![ScaffoldBody::new(body).into_widget()],
            background: None,
        }
    }

    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    pub fn app_bar(mut self, app_bar: AppBar) -> Self {
        self.children.insert(0, app_bar.into_widget());
        self
    }

    pub fn floating_action_button(mut self, fab: FloatingActionButton) -> Self {
        let body = self
            .children
            .pop()
            .expect("scaffold always has a body child");
        self.children
            .push(ScaffoldBody::new(body).with_fab(fab).into_widget());
        self
    }
}

impl Widget for Scaffold {
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
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        ctx.fill_rect(
            ctx.bounds(),
            self.background
                .unwrap_or_else(|| current_theme().colors.background),
        );
    }

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
    use crate::{set_current_theme, set_current_viewport, Theme, ViewportInfo};

    #[test]
    fn app_bar_uses_theme_defaults() {
        set_current_theme(Theme::default());
        set_current_viewport(ViewportInfo::default());

        let bar = AppBar::new("Hello");
        let style = bar.style();
        assert_eq!(style.size.width, percent(1.0));
        assert_eq!(style.size.height, length(56.0));
        assert_eq!(
            bar.accessibility_info().and_then(|info| info.label),
            Some("Hello".into())
        );
    }

    #[test]
    fn scaffold_orders_shell_children() {
        let scaffold = Scaffold::new(crate::Text::builder().content("Body").build())
            .app_bar(AppBar::new("Title"))
            .floating_action_button(FloatingActionButton::new("+"));

        assert_eq!(scaffold.children().len(), 2);
        assert_eq!(scaffold.children()[0].style().size.height, length(56.0));
        assert_eq!(scaffold.children()[1].children().len(), 2);
        assert_eq!(scaffold.children()[1].style().flex_grow, 1.0);
        assert_eq!(scaffold.children()[1].children()[1].children().len(), 1);
        assert_eq!(
            scaffold.children()[1].children()[1].children()[0]
                .children()
                .len(),
            1
        );
        assert_eq!(
            scaffold.children()[1].children()[1].children()[0].children()[0]
                .children()
                .len(),
            1
        );
        assert!(
            scaffold.children()[1].children()[1].children()[0].children()[0].children()[0]
                .focusable()
        );
    }

    #[test]
    fn fab_defaults_to_circle_and_button_role() {
        let fab = FloatingActionButton::new("+").accessibility_label("Increment");
        let style = fab.resolved_style();
        assert_eq!(style.min_width, 56.0);
        assert_eq!(style.min_height, 56.0);
        assert_eq!(
            fab.accessibility_info().and_then(|info| info.label),
            Some(String::from("Increment"))
        );
    }
}
