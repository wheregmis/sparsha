use sparsha::core::{glam::vec2, Color, Rect};
use sparsha::input::InputEvent;
use sparsha::layout::{taffy::prelude::*, WidgetId};
use sparsha::prelude::*;
use sparsha::text::TextStyle;
use sparsha::widgets::{EventContext, PaintContext};

const PROBE_CARD_WIDTH: f32 = 360.0;
const PROBE_CARD_HEIGHT: f32 = 236.0;

fn main() -> Result<(), sparsha::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsha::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::builder()
        .title("Sparsha Layout Probe")
        .width(960)
        .height(640)
        .theme(Theme::light())
        .router(
            Router::builder()
                .routes(vec![Route::new("/", || {
                    component().render(layout_probe_app).call()
                })])
                .fallback("/")
                .build(),
        )
        .build()
        .run()
}

fn layout_probe_app(cx: &mut ComponentContext<'_>) -> LayoutGuide {
    let theme = cx.theme();
    let viewport = cx.viewport();
    let viewport_label = format!(
        "Viewport {:.0} x {:.0} ({:?})",
        viewport.width, viewport.height, viewport.class
    );

    LayoutGuide::new(
        PROBE_CARD_WIDTH,
        PROBE_CARD_HEIGHT,
        Container::column()
            .fill()
            .main_axis_alignment(MainAxisAlignment::Center)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .background(theme.colors.background)
            .child(
                Container::column()
                    .size(PROBE_CARD_WIDTH, PROBE_CARD_HEIGHT)
                    .padding(28.0)
                    .gap(16.0)
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .background(theme.colors.surface)
                    .corner_radius(22.0)
                    .border(2.0, theme.colors.primary)
                    .child(
                        Text::builder()
                            .content("Centered Probe")
                            .font_size(30.0)
                            .bold(true)
                            .color(theme.colors.text_primary)
                            .build(),
                    )
                    .child(
                        Text::builder()
                            .content(viewport_label)
                            .font_size(14.0)
                            .color(theme.colors.text_muted)
                            .build(),
                    )
                    .child(
                        Text::builder()
                            .content("The blue card should sit exactly on the crosshair.")
                            .font_size(15.0)
                            .color(theme.colors.text_primary)
                            .align(TextAlign::Center)
                            .build(),
                    ),
            ),
    )
}

struct LayoutGuide {
    id: WidgetId,
    target_width: f32,
    target_height: f32,
    child: Box<dyn Widget>,
}

impl LayoutGuide {
    fn new(target_width: f32, target_height: f32, child: impl IntoWidget) -> Self {
        Self {
            id: WidgetId::default(),
            target_width,
            target_height,
            child: child.into_widget(),
        }
    }
}

fn scale_rect(bounds: Rect, scale_factor: f32) -> Rect {
    Rect::new(
        bounds.x * scale_factor,
        bounds.y * scale_factor,
        bounds.width * scale_factor,
        bounds.height * scale_factor,
    )
}

fn format_rect(bounds: Rect) -> String {
    format!(
        "x {:.0} y {:.0}  w {:.0} h {:.0}",
        bounds.x, bounds.y, bounds.width, bounds.height
    )
}

fn expand_rect(bounds: Rect, amount: f32) -> Rect {
    Rect::new(
        bounds.x - amount,
        bounds.y - amount,
        bounds.width + amount * 2.0,
        bounds.height + amount * 2.0,
    )
}

fn draw_dashed_rect(ctx: &mut PaintContext, bounds: Rect, color: Color, dash: f32, gap: f32) {
    let mut x = bounds.x;
    while x < bounds.x + bounds.width {
        let segment_end = (x + dash).min(bounds.x + bounds.width);
        ctx.stroke_line(vec2(x, bounds.y), vec2(segment_end, bounds.y), 1.0, color);
        ctx.stroke_line(
            vec2(x, bounds.y + bounds.height),
            vec2(segment_end, bounds.y + bounds.height),
            1.0,
            color,
        );
        x += dash + gap;
    }

    let mut y = bounds.y;
    while y < bounds.y + bounds.height {
        let segment_end = (y + dash).min(bounds.y + bounds.height);
        ctx.stroke_line(vec2(bounds.x, y), vec2(bounds.x, segment_end), 1.0, color);
        ctx.stroke_line(
            vec2(bounds.x + bounds.width, y),
            vec2(bounds.x + bounds.width, segment_end),
            1.0,
            color,
        );
        y += dash + gap;
    }
}

fn draw_corner_markers(ctx: &mut PaintContext, bounds: Rect, color: Color, arm: f32, inset: f32) {
    let left = bounds.x - inset;
    let right = bounds.x + bounds.width + inset;
    let top = bounds.y - inset;
    let bottom = bounds.y + bounds.height + inset;

    ctx.stroke_line(vec2(left, top), vec2(left + arm, top), 2.0, color);
    ctx.stroke_line(vec2(left, top), vec2(left, top + arm), 2.0, color);

    ctx.stroke_line(vec2(right - arm, top), vec2(right, top), 2.0, color);
    ctx.stroke_line(vec2(right, top), vec2(right, top + arm), 2.0, color);

    ctx.stroke_line(vec2(left, bottom), vec2(left + arm, bottom), 2.0, color);
    ctx.stroke_line(vec2(left, bottom - arm), vec2(left, bottom), 2.0, color);

    ctx.stroke_line(vec2(right - arm, bottom), vec2(right, bottom), 2.0, color);
    ctx.stroke_line(vec2(right, bottom - arm), vec2(right, bottom), 2.0, color);
}

fn probe_card_id(root: &dyn Widget) -> WidgetId {
    root.children()
        .first()
        .map(|child| child.id())
        .unwrap_or_else(|| root.id())
}

impl Widget for LayoutGuide {
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
            align_items: Some(AlignItems::Stretch),
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let center_x = bounds.x + bounds.width * 0.5;
        let center_y = bounds.y + bounds.height * 0.5;
        let guide = Color::from_hex(0x93C5FD).with_alpha(0.58);
        let target_border = Color::from_hex(0x0F766E);

        ctx.stroke_line(
            vec2(center_x, bounds.y),
            vec2(center_x, bounds.y + bounds.height),
            1.0,
            guide,
        );
        ctx.stroke_line(
            vec2(bounds.x, center_y),
            vec2(bounds.x + bounds.width, center_y),
            1.0,
            guide,
        );

        let target_width = self.target_width * ctx.scale_factor;
        let target_height = self.target_height * ctx.scale_factor;
        let target_rect = Rect::new(
            center_x - target_width * 0.5,
            center_y - target_height * 0.5,
            target_width,
            target_height,
        );
        ctx.fill_rounded_rect(
            Rect::new(center_x - 6.0, center_y - 6.0, 12.0, 12.0),
            target_border,
            6.0,
        );

        let probe_card_id = probe_card_id(self.child.as_ref());
        let card_rect = ctx
            .layout_tree
            .get_absolute_layout(probe_card_id)
            .map(|layout| scale_rect(layout.bounds, ctx.scale_factor));
        let delta = card_rect.map(|card_rect| {
            (
                card_rect.x - target_rect.x,
                card_rect.y - target_rect.y,
                card_rect.width - target_rect.width,
                card_rect.height - target_rect.height,
            )
        });

        let hud = Rect::new(bounds.x + 22.0, bounds.y + 20.0, 420.0, 176.0);
        ctx.fill_bordered_rect(
            hud,
            Color::WHITE.with_alpha(0.86),
            16.0,
            1.0,
            guide.with_alpha(0.65),
        );

        let title = TextStyle::new()
            .with_family("Inter")
            .with_size(16.0)
            .with_color(Color::from_hex(0x0F172A))
            .bold();
        let body = TextStyle::new()
            .with_family("Inter")
            .with_size(13.0)
            .with_color(Color::from_hex(0x475569));
        let metric = TextStyle::new()
            .with_family("JetBrains Mono")
            .with_size(12.0)
            .with_color(Color::from_hex(0x334155));

        ctx.draw_text("Layout Probe", &title, hud.x + 16.0, hud.y + 16.0);
        ctx.draw_text(
            "Teal guide = expected centered card position.",
            &body,
            hud.x + 16.0,
            hud.y + 44.0,
        );
        ctx.draw_text(
            "Orange outline = actual card bounds from the layout tree.",
            &body,
            hud.x + 16.0,
            hud.y + 66.0,
        );
        ctx.draw_text(
            &format!(
                "paint bounds   {}  @ {:.2}x",
                format_rect(bounds),
                ctx.scale_factor
            ),
            &metric,
            hud.x + 16.0,
            hud.y + 96.0,
        );
        ctx.draw_text(
            &format!("target bounds  {}", format_rect(target_rect)),
            &metric,
            hud.x + 16.0,
            hud.y + 116.0,
        );
        if let Some(card_rect) = card_rect {
            ctx.draw_text(
                &format!("card bounds    {}", format_rect(card_rect)),
                &metric,
                hud.x + 16.0,
                hud.y + 136.0,
            );
        }
        if let Some((dx, dy, dw, dh)) = delta {
            ctx.draw_text(
                &format!(
                    "delta          dx {:+.0} dy {:+.0} dw {:+.0} dh {:+.0}",
                    dx, dy, dw, dh
                ),
                &metric,
                hud.x + 16.0,
                hud.y + 156.0,
            );
        }
    }

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        std::slice::from_ref(&self.child)
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        std::slice::from_mut(&mut self.child)
    }

    fn paint_after_children(&self, ctx: &mut PaintContext) {
        let probe_card_id = probe_card_id(self.child.as_ref());
        let Some(card_layout) = ctx.layout_tree.get_absolute_layout(probe_card_id) else {
            return;
        };

        let bounds = ctx.bounds();
        let center_x = bounds.x + bounds.width * 0.5;
        let center_y = bounds.y + bounds.height * 0.5;
        let target_rect = Rect::new(
            center_x - self.target_width * ctx.scale_factor * 0.5,
            center_y - self.target_height * ctx.scale_factor * 0.5,
            self.target_width * ctx.scale_factor,
            self.target_height * ctx.scale_factor,
        );
        let target_border = Color::from_hex(0x0F766E);
        let actual_border = Color::from_hex(0xF97316);
        let card_rect = scale_rect(card_layout.bounds, ctx.scale_factor);

        draw_dashed_rect(
            ctx,
            expand_rect(target_rect, 10.0),
            target_border.with_alpha(0.75),
            12.0,
            8.0,
        );
        draw_corner_markers(ctx, target_rect, target_border, 18.0, 10.0);
        ctx.fill_bordered_rect(
            card_rect,
            actual_border.with_alpha(0.04),
            22.0,
            2.0,
            actual_border,
        );
    }
}
