use serde_json::json;
use spark::core::glam::Vec2;
use spark::input::PointerButton;
use spark::layout::{taffy, WidgetId};
use spark::prelude::*;
use spark::text::TextStyle;
use spark::widgets::{EventContext, PaintContext};
use std::f32::consts::{PI, TAU};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_arch = "wasm32")]
use web_time::{SystemTime, UNIX_EPOCH};

const TICK_MS: u64 = 33;

fn main() {
    #[cfg(target_arch = "wasm32")]
    spark::init_web();

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::new()
        .with_title("Fractal Clock - Spark")
        .with_size(1440, 960)
        .with_background(Color::from_hex(0x04060A))
        .run(|| Box::new(FractalClock::new()));
}

struct FractalClock {
    id: WidgetId,
    task_runtime: TaskRuntime,
    now_utc: Signal<f64>,
    pointer: Signal<Vec2>,
    zoom: Signal<f32>,
    palette_index: Signal<usize>,
}

#[derive(Clone, Copy)]
struct Palette {
    name: &'static str,
    bg: Color,
    veil: Color,
    mist: Color,
    minute: Color,
    hour: Color,
    second: Color,
    text: Color,
    dim: Color,
}

impl FractalClock {
    fn new() -> Self {
        let task_runtime = TaskRuntime::current_or_default();
        let now_utc = Signal::new(current_utc_seconds());
        let runtime_for_tick = task_runtime.clone();
        let now_for_results = now_utc;

        task_runtime.on_result(move |result| {
            if result.task_kind != "sleep_echo" {
                return;
            }
            now_for_results.set(current_utc_seconds());
            schedule_tick(&runtime_for_tick);
        });

        schedule_tick(&task_runtime);

        Self {
            id: WidgetId::default(),
            task_runtime,
            now_utc,
            pointer: Signal::new(Vec2::ZERO),
            zoom: Signal::new(1.0),
            palette_index: Signal::new(0),
        }
    }

    fn palette(&self) -> Palette {
        palette(self.palette_index.get())
    }

    fn digital_time(&self, seconds: f64) -> String {
        let total = (seconds.floor() as u64) % 86_400;
        let hours = total / 3_600;
        let minutes = (total / 60) % 60;
        let secs = total % 60;
        format!("{hours:02}:{minutes:02}:{secs:02} UTC")
    }

    fn draw_scene(&self, ctx: &mut PaintContext, palette: Palette, seconds: f64) {
        let bounds = ctx.bounds();
        let size = bounds.size();
        let center = bounds.center()
            + Vec2::new(
                self.pointer.get().x * size.x * 0.035,
                self.pointer.get().y * size.y * 0.035,
            );
        let zoom = self.zoom.get();
        let orbit_radius = size.x.min(size.y) * 0.18 * zoom;
        let total = seconds.rem_euclid(86_400.0);
        let hours_24 = (total / 3_600.0).floor() as u32;
        let minutes = ((total / 60.0).floor() as u32) % 60;
        let secs = total % 60.0;

        let hour_angle =
            (TAU * ((((hours_24 % 12) as f32) + minutes as f32 / 60.0 + secs as f32 / 3_600.0)
                / 12.0))
                - PI * 0.5;
        let minute_angle = (TAU * ((minutes as f32 + secs as f32 / 60.0) / 60.0)) - PI * 0.5;
        let second_angle = (TAU * (secs as f32 / 60.0)) - PI * 0.5;
        let drift = seconds as f32 * 0.08;
        let ambient = 0.55 + 0.45 * (seconds as f32 * 0.33).sin().abs();

        ctx.fill_rect(bounds, palette.bg);
        self.draw_backdrop(ctx, bounds, palette, drift);
        self.draw_stars(ctx, bounds, palette, drift);
        self.draw_orbit(ctx, center, orbit_radius, palette, drift);
        self.draw_hour_markers(ctx, center, orbit_radius * 1.18, palette, drift);

        self.draw_fractal_hand(
            ctx,
            center,
            hour_angle,
            orbit_radius * 0.86,
            6,
            5.6,
            palette.hour,
            palette.minute,
            drift * 0.8,
            ambient * 0.9,
            1.0,
        );
        self.draw_fractal_hand(
            ctx,
            center,
            minute_angle,
            orbit_radius * 1.08,
            7,
            4.5,
            palette.minute,
            palette.second,
            drift * 1.1 + 2.2,
            ambient,
            0.8,
        );
        self.draw_fractal_hand(
            ctx,
            center,
            second_angle,
            orbit_radius * 1.24,
            6,
            3.2,
            palette.second,
            palette.hour,
            drift * 1.7 + 4.4,
            1.0,
            0.45,
        );

        for orbit in 0..3 {
            let orbit_phase = drift * (1.0 + orbit as f32 * 0.23);
            let radius = orbit_radius * (1.45 + orbit as f32 * 0.21);
            self.draw_satellites(ctx, center, radius, palette, orbit_phase, orbit as f32);
        }

        self.draw_core(ctx, center, orbit_radius, palette, drift);
        self.draw_hud(ctx, bounds, palette, seconds, zoom, ambient);
    }

    fn draw_backdrop(&self, ctx: &mut PaintContext, bounds: Rect, palette: Palette, drift: f32) {
        let w = bounds.width;
        let h = bounds.height;
        for i in 0..18 {
            let band = i as f32 / 17.0;
            let phase = drift * 0.55 + band * 3.8;
            let x = bounds.x + w * (0.5 + 0.4 * (phase.sin() * 0.65 + (phase * 1.7).cos() * 0.2));
            let width = w * (0.04 + 0.028 * hash(i as f32 + 10.0));
            let color = mix_color(palette.veil, palette.mist, band).with_alpha(0.026 + band * 0.018);
            ctx.fill_rect(Rect::new(x - width * 0.5, bounds.y, width, h), color);
        }

        let vignette = 36.0;
        ctx.fill_rect(
            Rect::new(bounds.x, bounds.y, bounds.width, vignette),
            palette.bg.with_alpha(0.34),
        );
        ctx.fill_rect(
            Rect::new(bounds.x, bounds.y + bounds.height - vignette, bounds.width, vignette),
            palette.bg.with_alpha(0.42),
        );
    }

    fn draw_stars(&self, ctx: &mut PaintContext, bounds: Rect, palette: Palette, drift: f32) {
        for i in 0..160 {
            let seed = i as f32 * 1.371;
            let x = bounds.x + hash(seed + 1.1) * bounds.width;
            let y = bounds.y + hash(seed + 2.7) * bounds.height;
            let shimmer = 0.25 + 0.75 * ((drift * 1.8 + seed).sin() * 0.5 + 0.5);
            let parallax = Vec2::new(
                self.pointer.get().x * 4.0 * hash(seed + 7.0),
                self.pointer.get().y * 4.0 * hash(seed + 9.0),
            );
            let pos = Vec2::new(x, y) + parallax;
            let size = 0.8 + hash(seed + 5.4) * 2.1;
            let color = mix_color(palette.second, palette.minute, hash(seed + 8.1))
                .with_alpha(0.06 + 0.16 * shimmer);
            ctx.fill_rect(Rect::new(pos.x, pos.y, size, size), color);
        }
    }

    fn draw_orbit(
        &self,
        ctx: &mut PaintContext,
        center: Vec2,
        radius: f32,
        palette: Palette,
        drift: f32,
    ) {
        for i in 0..220 {
            let t = i as f32 / 220.0;
            let angle = t * TAU;
            let wobble = 1.0 + 0.025 * (drift * 2.0 + angle * 6.0).sin();
            let pos = center + direction(angle) * radius * wobble;
            let size = if i % 5 == 0 { 3.8 } else { 2.2 };
            let alpha = if i % 5 == 0 { 0.2 } else { 0.09 };
            let color = mix_color(palette.minute, palette.hour, t).with_alpha(alpha);
            ctx.fill_rect(
                Rect::new(pos.x - size * 0.5, pos.y - size * 0.5, size, size),
                color,
            );
        }
    }

    fn draw_hour_markers(
        &self,
        ctx: &mut PaintContext,
        center: Vec2,
        radius: f32,
        palette: Palette,
        drift: f32,
    ) {
        for marker in 0..12 {
            let angle = marker as f32 / 12.0 * TAU - PI * 0.5;
            let start = center + direction(angle) * (radius - 14.0);
            let end = center + direction(angle) * (radius + 16.0);
            let pulse = 0.55 + 0.45 * (drift * 1.4 + marker as f32 * 0.7).sin().abs();
            self.draw_segment(
                ctx,
                start,
                end,
                2.6,
                palette.second.with_alpha(0.045 * pulse),
            );
            self.draw_segment(
                ctx,
                start,
                end,
                1.2,
                mix_color(palette.hour, palette.minute, marker as f32 / 11.0)
                    .with_alpha(0.24 * pulse),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_fractal_hand(
        &self,
        ctx: &mut PaintContext,
        origin: Vec2,
        angle: f32,
        length: f32,
        depth: u32,
        thickness: f32,
        primary: Color,
        secondary: Color,
        drift: f32,
        ambient: f32,
        spread_scale: f32,
    ) {
        self.draw_branch(
            ctx,
            origin,
            angle,
            length,
            depth,
            thickness,
            primary,
            secondary,
            drift,
            ambient,
            spread_scale,
            0.0,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_branch(
        &self,
        ctx: &mut PaintContext,
        origin: Vec2,
        angle: f32,
        length: f32,
        depth: u32,
        thickness: f32,
        primary: Color,
        secondary: Color,
        drift: f32,
        ambient: f32,
        spread_scale: f32,
        seed: f32,
    ) {
        if depth == 0 || length < 7.0 {
            return;
        }

        let sway = (drift * 1.2 + seed * 1.7).sin() * 0.14;
        let end = origin + direction(angle + sway) * length;
        let pulse = 0.72 + 0.28 * (drift * 2.5 + seed * 0.9).cos().abs();
        let glow = mix_color(primary, secondary, 0.35 + 0.25 * (seed * 0.7).sin());
        let crisp = mix_color(primary, Color::WHITE, 0.22 + 0.08 * ambient);

        self.draw_segment(
            ctx,
            origin,
            end,
            thickness * 2.4,
            glow.with_alpha(0.028 * pulse),
        );
        self.draw_segment(
            ctx,
            origin,
            end,
            thickness * 1.25,
            glow.with_alpha(0.14 * pulse),
        );
        self.draw_segment(
            ctx,
            origin,
            end,
            thickness * 0.58,
            crisp.with_alpha(0.26 * pulse),
        );

        if depth == 1 {
            return;
        }

        let split = (0.24 + 0.06 * depth as f32) * spread_scale;
        let bend = 0.07 * (drift + seed * 3.1).sin();
        let child_length = length * (0.72 - 0.02 * (6.0 - depth as f32).max(0.0));
        let child_thickness = thickness * 0.8;
        let child_primary = mix_color(primary, secondary, 0.18);
        let child_secondary = mix_color(secondary, primary, 0.42);

        self.draw_branch(
            ctx,
            end,
            angle + split + bend,
            child_length,
            depth - 1,
            child_thickness,
            child_primary,
            child_secondary,
            drift + 0.31,
            ambient,
            spread_scale,
            seed + 1.0,
        );
        self.draw_branch(
            ctx,
            end,
            angle - split + bend * 0.7,
            child_length * 0.94,
            depth - 1,
            child_thickness * 0.96,
            child_primary,
            child_secondary,
            drift + 0.47,
            ambient,
            spread_scale,
            seed + 2.0,
        );
    }

    fn draw_satellites(
        &self,
        ctx: &mut PaintContext,
        center: Vec2,
        radius: f32,
        palette: Palette,
        phase: f32,
        seed: f32,
    ) {
        for i in 0..14 {
            let t = i as f32 / 14.0;
            let angle = phase * (0.85 + seed * 0.07) + t * TAU + seed;
            let pos = center + direction(angle) * radius;
            let size = 2.0 + 3.0 * ((phase + t * 5.0).sin() * 0.5 + 0.5);
            let color = mix_color(palette.minute, palette.hour, t).with_alpha(0.085);
            ctx.fill_rect(
                Rect::new(pos.x - size * 0.5, pos.y - size * 0.5, size, size),
                color,
            );
        }
    }

    fn draw_core(
        &self,
        ctx: &mut PaintContext,
        center: Vec2,
        radius: f32,
        palette: Palette,
        drift: f32,
    ) {
        for layer in 0..5 {
            let t = layer as f32 / 4.0;
            let size = radius * (0.19 - t * 0.028);
            let pulse = 0.8 + 0.2 * (drift * 3.4 + layer as f32).sin().abs();
            let color = mix_color(palette.hour, palette.minute, t).with_alpha(0.028 * pulse);
            ctx.fill_rect(
                Rect::new(center.x - size, center.y - size, size * 2.0, size * 2.0),
                color,
            );
        }

        let nucleus = radius * 0.078;
        ctx.fill_rect(
            Rect::new(
                center.x - nucleus,
                center.y - nucleus,
                nucleus * 2.0,
                nucleus * 2.0,
            ),
            palette.second.with_alpha(0.24),
        );
        ctx.fill_rect(
            Rect::new(
                center.x - nucleus * 0.42,
                center.y - nucleus * 0.42,
                nucleus * 0.84,
                nucleus * 0.84,
            ),
            palette.second.with_alpha(0.68),
        );
    }

    fn draw_hud(
        &self,
        ctx: &mut PaintContext,
        bounds: Rect,
        palette: Palette,
        seconds: f64,
        zoom: f32,
        ambient: f32,
    ) {
        let title = TextStyle::new()
            .with_family("Inter")
            .with_size(15.0)
            .with_color(palette.text)
            .bold();
        let body = TextStyle::new()
            .with_family("Inter")
            .with_size(12.0)
            .with_color(palette.dim);
        let hero = TextStyle::new()
            .with_family("Inter")
            .with_size(30.0)
            .with_color(palette.text)
            .bold();

        let info_panel = Rect::new(bounds.x + 26.0, bounds.y + 24.0, 250.0, 108.0);
        ctx.fill_bordered_rect(
            info_panel,
            palette.veil.with_alpha(0.62),
            16.0,
            1.0,
            palette.second.with_alpha(0.08),
        );
        ctx.draw_text("FRACTAL CLOCK", &title, info_panel.x + 16.0, info_panel.y + 16.0);
        ctx.draw_text(
            palette.name,
            &body.clone().with_color(palette.minute),
            info_panel.x + 16.0,
            info_panel.y + 42.0,
        );
        ctx.draw_text(
            "Primary click shifts palette",
            &body,
            info_panel.x + 16.0,
            info_panel.y + 64.0,
        );
        ctx.draw_text(
            "Scroll adjusts orbit density",
            &body,
            info_panel.x + 16.0,
            info_panel.y + 84.0,
        );

        let time_panel = Rect::new(bounds.x + 26.0, bounds.y + bounds.height - 110.0, 320.0, 72.0);
        ctx.fill_bordered_rect(
            time_panel,
            palette.veil.with_alpha(0.66),
            18.0,
            1.0,
            palette.second.with_alpha(0.08),
        );
        ctx.draw_text(
            &self.digital_time(seconds),
            &hero,
            time_panel.x + 18.0,
            time_panel.y + 18.0,
        );
        ctx.draw_text(
            "Time source: UTC lattice / live task ticker",
            &body,
            time_panel.x + 20.0,
            time_panel.y + 52.0,
        );

        let stats = format!(
            "Zoom {:>4.0}%   Pulse {:>3.0}%   Worker {}",
            zoom * 100.0,
            ambient * 100.0,
            if self.task_runtime.has_in_flight() { "active" } else { "idle" }
        );
        let stats_style = body.with_color(palette.hour);
        let (stats_width, _) = ctx.measure_text(&stats, &stats_style);
        let stats_x = bounds.x + bounds.width - stats_width - 28.0;
        let stats_y = bounds.y + bounds.height - 42.0;
        ctx.draw_text(&stats, &stats_style, stats_x, stats_y);
    }

    fn draw_segment(
        &self,
        ctx: &mut PaintContext,
        start: Vec2,
        end: Vec2,
        thickness: f32,
        color: Color,
    ) {
        let delta = end - start;
        let distance = delta.length();
        if distance <= 0.001 {
            return;
        }

        let steps = ((distance / (thickness.max(1.0) * 1.65)).ceil() as usize).clamp(1, 26);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let pos = start.lerp(end, t);
            let taper = 0.86 - (t - 0.5).abs() * 0.24;
            let size = (thickness * taper).max(0.9);
            ctx.fill_rect(
                Rect::new(pos.x - size * 0.5, pos.y - size * 0.5, size, size),
                color,
            );
        }
    }
}

impl Widget for FractalClock {
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
        let palette = self.palette();
        self.draw_scene(ctx, palette, self.now_utc.get());
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
            InputEvent::PointerDown { button, .. } if *button == PointerButton::Primary => {
                self.palette_index.set((self.palette_index.get() + 1) % palette_count());
                ctx.request_paint();
            }
            InputEvent::Scroll { delta, .. } => {
                let next_zoom = (self.zoom.get() + delta.y * 0.015).clamp(0.72, 1.45);
                self.zoom.set(next_zoom);
                ctx.request_paint();
            }
            _ => {}
        }
    }
}

fn schedule_tick(runtime: &TaskRuntime) {
    runtime.spawn("sleep_echo", json!({ "millis": TICK_MS, "data": null }));
}

fn current_utc_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

fn direction(angle: f32) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin())
}

fn hash(seed: f32) -> f32 {
    let value = (seed.sin() * 43_758.547).abs();
    value - value.floor()
}

fn palette_count() -> usize {
    3
}

fn palette(index: usize) -> Palette {
    match index % palette_count() {
        0 => Palette {
            name: "Solar Drift",
            bg: Color::from_hex(0x04060A),
            veil: Color::from_hex(0x09131F),
            mist: Color::from_hex(0x12344A),
            minute: Color::from_hex(0x47D1C8),
            hour: Color::from_hex(0xF39C5A),
            second: Color::from_hex(0xF4F1E8),
            text: Color::from_hex(0xE6F0FF),
            dim: Color::from_hex(0x86A3B8),
        },
        1 => Palette {
            name: "Noctiluca",
            bg: Color::from_hex(0x05070D),
            veil: Color::from_hex(0x111A29),
            mist: Color::from_hex(0x1F2C46),
            minute: Color::from_hex(0x6BE9C3),
            hour: Color::from_hex(0xF06767),
            second: Color::from_hex(0xFFF7E6),
            text: Color::from_hex(0xF4F8FF),
            dim: Color::from_hex(0x8EA0B7),
        },
        _ => Palette {
            name: "Aurora Brass",
            bg: Color::from_hex(0x030406),
            veil: Color::from_hex(0x11161C),
            mist: Color::from_hex(0x1D2A23),
            minute: Color::from_hex(0x82D66F),
            hour: Color::from_hex(0xD7B15A),
            second: Color::from_hex(0xFBFAF6),
            text: Color::from_hex(0xF2F4ED),
            dim: Color::from_hex(0x9DA88C),
        },
    }
}

fn mix_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}
