use sparsh::core::glam::Vec2;
use sparsh::layout::{taffy, WidgetId};
use sparsh::prelude::*;
use sparsh::text::TextStyle;
use sparsh::widgets::{DrawSurfaceContext, EventContext, PaintContext};
use std::f32::consts::TAU;

fn main() -> Result<(), sparsh::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsh::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::new()
        .title("Hybrid Overlay - Sparsh")
        .size(1280, 800)
        .background(Color::from_hex(0x07111D))
        .theme(Theme::light())
        .router(
            Router::new()
                .route("/", || Box::new(HybridOverlayDemo::new()))
                .fallback("/"),
        )
        .run()
}

struct HybridOverlayDemo {
    id: WidgetId,
    pointer: Signal<Vec2>,
    accent_index: Signal<usize>,
    surface: DrawSurface,
}

struct HybridOverlayScene {
    pointer: Signal<Vec2>,
    accent_index: Signal<usize>,
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    panel: Color,
    panel_border: Color,
    text: Color,
    subtext: Color,
    primary: Color,
    secondary: Color,
    glow: Color,
}

impl HybridOverlayDemo {
    fn new() -> Self {
        let pointer = Signal::new(Vec2::ZERO);
        let accent_index = Signal::new(0usize);
        let scene = HybridOverlayScene {
            pointer,
            accent_index,
        };
        Self {
            id: WidgetId::default(),
            pointer,
            accent_index,
            surface: DrawSurface::new(move |ctx| scene.paint(ctx)).fill(),
        }
    }

    fn palette(&self) -> Palette {
        palette(self.accent_index.get())
    }

    fn paint_overlay(&self, ctx: &mut PaintContext) {
        let bounds = ctx.bounds();
        let palette = self.palette();
        let title = TextStyle::new()
            .with_family("Inter")
            .with_size(14.0)
            .with_color(palette.text)
            .bold();
        let body = TextStyle::new()
            .with_family("Inter")
            .with_size(12.0)
            .with_color(palette.subtext);
        let metric = TextStyle::new()
            .with_family("Inter")
            .with_size(28.0)
            .with_color(palette.text)
            .bold();

        let info = Rect::new(bounds.x + 28.0, bounds.y + 28.0, 320.0, 152.0);
        ctx.fill_bordered_rect(info, palette.panel, 20.0, 1.0, palette.panel_border);
        ctx.draw_text("HYBRID OVERLAY", &title, info.x + 20.0, info.y + 20.0);
        ctx.draw_text(
            "GPU scene lives inside DrawSurface.",
            &body,
            info.x + 20.0,
            info.y + 56.0,
        );
        ctx.draw_text(
            "Panels and text stay on the retained DOM path.",
            &body,
            info.x + 20.0,
            info.y + 80.0,
        );
        ctx.draw_text(
            "Move pointer to steer the flow. Click to cycle accents.",
            &body,
            info.x + 20.0,
            info.y + 104.0,
        );

        let badge = Rect::new(
            bounds.x + 28.0,
            bounds.y + bounds.height - 116.0,
            364.0,
            72.0,
        );
        ctx.fill_bordered_rect(badge, palette.panel, 22.0, 1.0, palette.panel_border);
        ctx.draw_text("DOM + GPU", &metric, badge.x + 20.0, badge.y + 16.0);
        ctx.draw_text(
            "Hybrid rendering without giving up retained overlays",
            &body,
            badge.x + 22.0,
            badge.y + 50.0,
        );

        let status = format!(
            "Pointer {:>3.0}% {:>3.0}%   Accent {}",
            (self.pointer.get().x * 50.0 + 50.0).clamp(0.0, 100.0),
            (self.pointer.get().y * 50.0 + 50.0).clamp(0.0, 100.0),
            self.accent_index.get() + 1,
        );
        let status_style = body.clone().with_color(palette.primary);
        let (status_width, _) = ctx.measure_text(&status, &status_style);
        ctx.draw_text(
            &status,
            &status_style,
            bounds.x + bounds.width - status_width - 32.0,
            bounds.y + bounds.height - 44.0,
        );
    }
}

impl HybridOverlayScene {
    fn palette(&self) -> Palette {
        palette(self.accent_index.get())
    }

    fn paint(&self, ctx: &mut DrawSurfaceContext) {
        ctx.request_next_frame();
        let palette = self.palette();
        let bounds = ctx.bounds;
        let center = bounds.center()
            + Vec2::new(
                self.pointer.get().x * bounds.width * 0.08,
                self.pointer.get().y * bounds.height * 0.08,
            );
        let radius = bounds.width.min(bounds.height) * 0.22;
        let t = ctx.elapsed_time;

        ctx.fill_rect(bounds, palette.background);

        for band in 0..12 {
            let phase = t * 0.18 + band as f32 * 0.4;
            let width = bounds.width * (0.12 + band as f32 * 0.01);
            let x = bounds.x + bounds.width * (0.5 + 0.3 * phase.sin());
            let color = mix(palette.glow, palette.secondary, band as f32 / 11.0)
                .with_alpha(0.012 + band as f32 * 0.004);
            ctx.fill_rect(
                Rect::new(x - width * 0.5, bounds.y, width, bounds.height),
                color,
            );
        }

        for ring in 0..3 {
            let ring_radius = radius * (1.0 + ring as f32 * 0.34);
            for i in 0..120 {
                let progress = i as f32 / 120.0;
                let angle = progress * TAU + t * (0.24 + ring as f32 * 0.08);
                let wobble = 1.0 + 0.05 * (t * 0.7 + progress * TAU * 3.0).sin();
                let pos = center + unit(angle) * ring_radius * wobble;
                let size = if i % 8 == 0 { 3.6 } else { 1.8 };
                let color = mix(palette.primary, palette.secondary, progress)
                    .with_alpha(if i % 8 == 0 { 0.18 } else { 0.07 });
                ctx.fill_rect(
                    Rect::new(pos.x - size * 0.5, pos.y - size * 0.5, size, size),
                    color,
                );
            }
        }

        for branch in 0..7 {
            let base_angle = t * 0.11 + branch as f32 / 7.0 * TAU;
            let start = center + unit(base_angle) * radius * 0.34;
            let end = center + unit(base_angle * 1.07) * radius * 1.3;
            let color = mix(palette.primary, palette.secondary, branch as f32 / 6.0);
            draw_branch(ctx, start, end, color, t + branch as f32 * 0.6, 4);
        }

        let core = radius * 0.08;
        ctx.fill_rect(
            Rect::new(center.x - core, center.y - core, core * 2.0, core * 2.0),
            palette.text.with_alpha(0.8),
        );
    }
}

impl Widget for HybridOverlayDemo {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> taffy::Style {
        taffy::Style {
            size: taffy::prelude::Size {
                width: taffy::prelude::percent(1.0),
                height: taffy::prelude::percent(1.0),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        self.surface.paint(ctx);
        self.paint_overlay(ctx);
    }

    fn draw_surface(&self) -> Option<&DrawSurface> {
        Some(&self.surface)
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) {
        match event {
            InputEvent::PointerMove { pos } => {
                let local = ctx.to_local(*pos);
                let bounds = ctx.bounds();
                if bounds.width > 0.0 && bounds.height > 0.0 {
                    let normalized = Vec2::new(
                        ((local.x / bounds.width) * 2.0 - 1.0).clamp(-1.0, 1.0),
                        ((local.y / bounds.height) * 2.0 - 1.0).clamp(-1.0, 1.0),
                    );
                    self.pointer.set(normalized);
                    ctx.request_paint();
                }
            }
            InputEvent::PointerDown { .. } => {
                self.accent_index.set((self.accent_index.get() + 1) % 3);
                ctx.request_paint();
            }
            _ => {}
        }
    }
}

fn draw_branch(
    ctx: &mut DrawSurfaceContext,
    start: Vec2,
    end: Vec2,
    color: Color,
    seed: f32,
    depth: u32,
) {
    let delta = end - start;
    let length = delta.length();
    if depth == 0 || length < 16.0 {
        return;
    }

    ctx.stroke_line(start, end, 2.4 + depth as f32 * 0.5, color.with_alpha(0.04));
    ctx.stroke_line(
        start,
        end,
        1.1 + depth as f32 * 0.24,
        color.with_alpha(0.28),
    );

    let angle = delta.y.atan2(delta.x);
    let next_length = length * 0.68;
    let mid = end;
    let spread = 0.34 + 0.05 * (seed * 0.8).sin();

    draw_branch(
        ctx,
        mid,
        mid + unit(angle + spread) * next_length,
        color,
        seed + 0.8,
        depth - 1,
    );
    draw_branch(
        ctx,
        mid,
        mid + unit(angle - spread * 0.9) * next_length * 0.92,
        color,
        seed + 1.4,
        depth - 1,
    );
}

fn unit(angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin())
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn palette(index: usize) -> Palette {
    match index % 3 {
        0 => Palette {
            background: Color::from_hex(0x07111D),
            panel: Color::from_hex(0x0B1624).with_alpha(0.82),
            panel_border: Color::from_hex(0x1C2C43).with_alpha(0.95),
            text: Color::from_hex(0xEFF7FF),
            subtext: Color::from_hex(0x8AA2BF),
            primary: Color::from_hex(0x5CE1D6),
            secondary: Color::from_hex(0xFF9F6E),
            glow: Color::from_hex(0x0E2F4B),
        },
        1 => Palette {
            background: Color::from_hex(0x0C0F16),
            panel: Color::from_hex(0x161B28).with_alpha(0.84),
            panel_border: Color::from_hex(0x2E3244).with_alpha(0.95),
            text: Color::from_hex(0xF5F4EF),
            subtext: Color::from_hex(0xAAA493),
            primary: Color::from_hex(0xB4F15D),
            secondary: Color::from_hex(0xF08C68),
            glow: Color::from_hex(0x233218),
        },
        _ => Palette {
            background: Color::from_hex(0x09060E),
            panel: Color::from_hex(0x17111E).with_alpha(0.84),
            panel_border: Color::from_hex(0x34253F).with_alpha(0.95),
            text: Color::from_hex(0xFFF6F7),
            subtext: Color::from_hex(0xB29AAA),
            primary: Color::from_hex(0xEA7CB9),
            secondary: Color::from_hex(0x7ACAF8),
            glow: Color::from_hex(0x2A1435),
        },
    }
}
