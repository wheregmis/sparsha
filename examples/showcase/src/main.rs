use sparsha::core::glam::Vec2;
use sparsha::layout::taffy::prelude::{length, percent, AlignItems, JustifyContent, Size, Style};
use sparsha::prelude::*;
use sparsha::text::TextStyle;
use sparsha::widgets::{current_theme, ButtonStyle, PaintContext, WidgetId};
use std::cell::RefCell;
use std::rc::Rc;

fn main() -> Result<(), sparsha::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsha::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let navigator_slot = Rc::new(RefCell::new(None::<Navigator>));
    let components_slot = navigator_slot.clone();
    let rendering_slot = navigator_slot.clone();

    let router = Router::new()
        .route("/components", move || {
            let navigator = components_slot
                .borrow()
                .clone()
                .expect("showcase navigator should be initialized before build");
            showcase_shell(ShowcaseRoute::Components, navigator)
        })
        .route("/rendering", move || {
            let navigator = rendering_slot
                .borrow()
                .clone()
                .expect("showcase navigator should be initialized before build");
            showcase_shell(ShowcaseRoute::Rendering, navigator)
        })
        .fallback("/components");

    *navigator_slot.borrow_mut() = Some(router.navigator());

    let theme = showcase_theme();
    let background = theme.colors.background;

    App::new()
        .title("Sparsha Showcase")
        .size(1440, 960)
        .background(background)
        .theme(theme)
        .router(router)
        .run()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowcaseRoute {
    Components,
    Rendering,
}

impl ShowcaseRoute {
    fn path(self) -> &'static str {
        match self {
            Self::Components => "/components",
            Self::Rendering => "/rendering",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Components => "Basic component preview",
            Self::Rendering => "Manual rendering checks",
        }
    }

    fn eyebrow(self) -> &'static str {
        match self {
            Self::Components => "COMPONENTS",
            Self::Rendering => "RENDERING",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Components => {
                "A compact read on the default widget vocabulary.\nThis page stays curated on purpose."
            }
            Self::Rendering => {
                "One screenshot should reveal line, stroke,\nand text issues on the web surface."
            }
        }
    }
}

fn showcase_theme() -> Theme {
    let mut theme = Theme::dark();
    theme.colors.background = Color::from_hex(0x16181D);
    theme.colors.surface = Color::from_hex(0x1D2128);
    theme.colors.surface_variant = Color::from_hex(0x252A33);
    theme.colors.surface_done = Color::from_hex(0x11161D);
    theme.colors.text_primary = Color::from_hex(0xE8EDF4);
    theme.colors.text_muted = Color::from_hex(0x96A0AE);
    theme.colors.primary = Color::from_hex(0x2385B9);
    theme.colors.primary_hovered = Color::from_hex(0x2F9AD2);
    theme.colors.primary_pressed = Color::from_hex(0x176A93);
    theme.colors.border = Color::from_hex(0x353B46);
    theme.colors.border_focus = Color::from_hex(0x72C5EE);
    theme.colors.disabled = Color::from_hex(0x586474);
    theme.colors.input_background = Color::from_hex(0x151A22);
    theme.colors.input_placeholder = Color::from_hex(0x6E7887);
    theme.typography.body_size = 15.0;
    theme.typography.title_size = 26.0;
    theme.controls.control_height = 36.0;
    theme.controls.scrollbar_thickness = 8.0;
    theme.radii.md = 8.0;
    theme.radii.lg = 14.0;
    theme
}

fn showcase_shell(route: ShowcaseRoute, navigator: Navigator) -> Container {
    let theme = current_theme();
    Container::new()
        .fill()
        .background(theme.colors.background)
        .column()
        .child(build_top_bar(route, navigator))
        .child(
            Container::new()
                .row()
                .fill()
                .child(build_sidebar(route))
                .child(build_main_area(route)),
        )
}

fn build_top_bar(route: ShowcaseRoute, navigator: Navigator) -> Container {
    let theme = current_theme();
    Container::new()
        .row()
        .fill_width()
        .padding(16.0)
        .background(theme.colors.surface_done)
        .border(1.0, theme.colors.border)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::SpaceBetween)
        .child(
            Container::new()
                .column()
                .gap(4.0)
                .child(Text::new("Sparsha Showcase").size(18.0).bold())
                .child(
                    Text::new("A page-ready preview surface for widgets and visual checks.")
                        .size(13.0)
                        .color(theme.colors.text_muted),
                ),
        )
        .child(
            Container::new()
                .row()
                .gap(10.0)
                .align_items(AlignItems::Center)
                .child(route_button(
                    "Components",
                    ShowcaseRoute::Components,
                    route == ShowcaseRoute::Components,
                    navigator.clone(),
                ))
                .child(route_button(
                    "Rendering",
                    ShowcaseRoute::Rendering,
                    route == ShowcaseRoute::Rendering,
                    navigator,
                )),
        )
}

fn route_button(
    label: &'static str,
    destination: ShowcaseRoute,
    active: bool,
    navigator: Navigator,
) -> Button {
    let theme = current_theme();
    let style = ButtonStyle {
        background: if active {
            theme.colors.primary_hovered
        } else {
            theme.colors.surface_variant
        },
        background_hovered: if active {
            theme.colors.primary
        } else {
            theme.colors.surface
        },
        background_pressed: if active {
            theme.colors.primary_pressed
        } else {
            theme.colors.surface_done
        },
        background_disabled: theme.colors.disabled,
        text_color: if active {
            Color::WHITE
        } else {
            theme.colors.text_primary
        },
        text_color_disabled: theme.colors.text_muted,
        border_color: if active {
            theme.colors.primary
        } else {
            theme.colors.border
        },
        border_width: 1.0,
        corner_radius: 10.0,
        padding_h: 14.0,
        padding_v: 8.0,
        font_size: 13.0,
        min_width: 120.0,
        min_height: 34.0,
    };

    Button::new(label)
        .with_style(style)
        .on_click(move || navigator.go(destination.path()))
}

fn build_sidebar(route: ShowcaseRoute) -> Container {
    let theme = current_theme();
    let sidebar = match route {
        ShowcaseRoute::Components => Container::new()
            .column()
            .gap(18.0)
            .child(sidebar_block(
                "In scope",
                &[
                    "Controls, type, editors, and viewport basics.",
                    "A fast read on the default design system.",
                ],
            ))
            .child(sidebar_block(
                "Manual checks",
                &[
                    "Tab through the checkbox and both editors.",
                    "Pan the wide sample and the virtualized list.",
                    "Switch tabs to confirm hash routing.",
                ],
            ))
            .child(sidebar_block(
                "Why this page",
                &[
                    "This is the public-facing preview surface.",
                    "Kitchen sink still handles broader regression work.",
                ],
            )),
        ShowcaseRoute::Rendering => Container::new()
            .column()
            .gap(18.0)
            .child(sidebar_block(
                "Look for",
                &[
                    "Crisp 1 px lines when you zoom.",
                    "A clip band that stays inside its frame.",
                    "Balanced text on dark and light swatches.",
                ],
            ))
            .child(sidebar_block(
                "Atlas tiles",
                &["Pixel alignment", "Stroke + clip", "Text rendering"],
            ))
            .child(sidebar_block(
                "Intent",
                &[
                    "One canvas, three quick reads.",
                    "Less prose, more visible signal.",
                ],
            )),
    };

    Container::new()
        .width(320.0)
        .fill_height()
        .background(theme.colors.surface)
        .border(1.0, theme.colors.border)
        .child(
            Scroll::new().vertical().fill_height().content(
                Container::new()
                    .column()
                    .gap(20.0)
                    .padding(20.0)
                    .child(
                        Text::new(route.eyebrow())
                            .size(11.0)
                            .bold()
                            .color(theme.colors.primary),
                    )
                    .child(Text::new(route.title()).size(22.0).bold())
                    .child(
                        Text::new(route.summary())
                            .size(13.0)
                            .color(theme.colors.text_muted),
                    )
                    .child(sidebar),
            ),
        )
}

fn sidebar_block(title: &'static str, lines: &[&'static str]) -> Container {
    let theme = current_theme();
    let mut block = Container::new()
        .column()
        .gap(10.0)
        .padding(14.0)
        .background(theme.colors.surface_variant)
        .border(1.0, theme.colors.border)
        .corner_radius(12.0)
        .child(Text::new(title).size(14.0).bold());

    for line in lines {
        block = block.child(
            Text::new(format!("• {}", line))
                .size(13.0)
                .color(theme.colors.text_muted),
        );
    }

    block
}

fn build_main_area(route: ShowcaseRoute) -> Scroll {
    Scroll::new()
        .vertical()
        .fill_height()
        .flex_grow(1.0)
        .content(match route {
            ShowcaseRoute::Components => build_components_page(),
            ShowcaseRoute::Rendering => build_rendering_page(),
        })
}

fn build_components_page() -> Container {
    Container::new()
        .column()
        .gap(20.0)
        .padding(24.0)
        .fill_width()
        .child(page_intro(
            ShowcaseRoute::Components,
            "The goal is a quick, reliable read on the default widget set.\nThe page favors intentional samples over a wall of controls.",
        ))
        .child(build_controls_card())
        .child(build_typography_card())
        .child(build_inputs_card())
        .child(build_viewport_card())
}

fn build_rendering_page() -> Container {
    Container::new()
        .column()
        .gap(14.0)
        .padding(24.0)
        .fill_width()
        .child(page_intro(
            ShowcaseRoute::Rendering,
            "A compact atlas for line, stroke, and text checks.\nIf the web surface is off, this page should make it obvious.",
        ))
        .child(rendering_hint_row())
        .child(rendering_atlas_card())
}

fn page_intro(route: ShowcaseRoute, detail: &'static str) -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(10.0)
        .padding(20.0)
        .background(theme.colors.surface_done)
        .border(1.0, theme.colors.border)
        .corner_radius(16.0)
        .child(
            Text::new(route.eyebrow())
                .size(11.0)
                .bold()
                .color(theme.colors.primary),
        )
        .child(Text::new(route.title()).size(28.0).bold())
        .child(Text::new(detail).size(14.0).color(theme.colors.text_muted))
}

fn section_card(
    title: &'static str,
    description: &'static str,
    content: impl Widget + 'static,
) -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(16.0)
        .padding(20.0)
        .background(theme.colors.surface)
        .border(1.0, theme.colors.border)
        .corner_radius(16.0)
        .child(Text::new(title).size(20.0).bold())
        .child(
            Text::new(description)
                .size(13.0)
                .color(theme.colors.text_muted),
        )
        .child(content)
}

fn build_controls_card() -> Container {
    let theme = current_theme();
    let secondary_style = ButtonStyle {
        background: theme.colors.surface_variant,
        background_hovered: theme.colors.surface_done,
        background_pressed: theme.colors.surface_done,
        background_disabled: theme.colors.disabled,
        text_color: theme.colors.text_primary,
        text_color_disabled: theme.colors.text_muted,
        border_color: theme.colors.border,
        border_width: 1.0,
        corner_radius: 10.0,
        padding_h: 14.0,
        padding_v: 8.0,
        font_size: 14.0,
        min_width: 148.0,
        min_height: 36.0,
    };

    section_card(
        "Controls",
        "Primary, secondary, and disabled actions using the shipped theme tokens,\nplus a single checkbox that stays in the normal focus order.",
        Container::new()
            .column()
            .gap(16.0)
            .child(
                Container::new()
                    .row()
                    .gap(12.0)
                    .wrap()
                    .child(Button::new("Primary Action").on_click(|| {}))
                    .child(
                        Button::new("Secondary Action")
                            .with_style(secondary_style)
                            .on_click(|| {}),
                    )
                    .child(Button::new("Disabled State").disabled(true)),
            )
            .child(
                Container::new()
                    .row()
                    .gap(12.0)
                    .align_items(AlignItems::Center)
                    .child(
                        Semantics::new(Checkbox::with_checked(true))
                            .label("Showcase interactive checkbox"),
                    )
                    .child(Text::new("Interactive checkbox").size(14.0)),
            )
            .child(
                Text::new(
                    "The goal is to show the default feel quickly.\nDeeper interaction coverage still lives in the other examples.",
                )
                .size(13.0)
                .color(theme.colors.text_muted),
            ),
    )
}

fn rendering_hint_row() -> Container {
    let theme = current_theme();
    Container::new()
        .row()
        .gap(12.0)
        .wrap()
        .fill_width()
        .child(rendering_hint_chip(
            "Pixel alignment",
            "Thin lines and square ramp stay sharp.",
        ))
        .child(rendering_hint_chip(
            "Stroke + clip",
            "Width ladder and clipped band stay clean.",
        ))
        .child(rendering_hint_chip(
            "Text rendering",
            "Dark and light swatches stay balanced.",
        ))
        .background(theme.colors.background)
}

fn rendering_hint_chip(title: &'static str, detail: &'static str) -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(6.0)
        .padding(14.0)
        .min_size(220.0, 0.0)
        .background(theme.colors.surface)
        .border(1.0, theme.colors.border)
        .corner_radius(12.0)
        .child(Text::new(title).size(14.0).bold())
        .child(Text::new(detail).size(12.0).color(theme.colors.text_muted))
}

fn build_typography_card() -> Container {
    let theme = current_theme();
    section_card(
        "Typography",
        "A small read on heading, body, and caption scales inside the same surface.\nThis makes the default rhythm easy to judge at a glance.",
        Container::new()
            .column()
            .gap(14.0)
            .child(
                Container::new()
                    .column()
                    .gap(10.0)
                    .padding(16.0)
                    .background(theme.colors.surface_variant)
                    .border(1.0, theme.colors.border)
                    .corner_radius(12.0)
                    .child(Text::header("Sparsh makes the default stack feel intentional."))
                    .child(
                        Text::new(
                            "Body copy should stay legible in denser panels without losing hierarchy.",
                        )
                        .size(15.0),
                    )
                    .child(Text::caption("Caption text keeps the secondary story out of the way.")),
            )
            .child(
                Text::new(
                    "Typography is doing the structural work here.\nThere is no extra ornament needed for the preview.",
                )
                .size(13.0)
                .color(theme.colors.text_muted),
            ),
    )
}

fn build_inputs_card() -> Container {
    let theme = current_theme();
    section_card(
        "Inputs",
        "Single-line and multiline editors should feel coherent with the same theme.\nLabels are explicit so browser smoke tests can target them directly.",
        Container::new()
            .column()
            .gap(16.0)
            .child(
                Semantics::new(
                    TextInput::new()
                        .fill_width()
                        .value("sparsh@example.dev")
                        .placeholder("Email address"),
                )
                .label("Showcase single-line input"),
            )
            .child(
                Semantics::new(
                    TextArea::new()
                        .fill_width()
                        .value(
                            "Static scenes make rendering bugs easier to spot.\nSmoke tests still probe the route and DOM surface.",
                        )
                        .placeholder("Notes"),
                )
                .label("Showcase multiline input"),
            )
            .child(
                Text::new(
                    "These fields use the same interaction model as the broader examples,\njust in a smaller, more curated setting.",
                )
                .size(13.0)
                .color(theme.colors.text_muted),
            ),
    )
}

fn build_viewport_card() -> Container {
    let theme = current_theme();
    section_card(
        "Viewport",
        "A wide two-axis scroll sample sits next to a compact virtualized list.\nThis keeps the page honest about the core viewport primitives.",
        Container::new()
            .column()
            .gap(16.0)
            .child(
                Container::new()
                    .row()
                    .gap(16.0)
                    .fill_width()
                    .child(build_scroll_sample().flex_grow(1.0).min_size(0.0, 250.0))
                    .child(build_virtual_list_sample().flex_grow(1.0).min_size(0.0, 250.0)),
            )
            .child(
                Text::new(
                    "The left sample should pan both ways. The right sample should recycle rows\ninstead of realizing the whole list at once.",
                )
                .size(13.0)
                .color(theme.colors.text_muted),
            ),
    )
}

fn build_scroll_sample() -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(10.0)
        .child(Text::new("Two-axis scroll").size(14.0).bold())
        .child(
            Container::new()
                .height(220.0)
                .background(theme.colors.surface_variant)
                .border(1.0, theme.colors.border)
                .corner_radius(12.0)
                .child(
                    Semantics::new(
                        Scroll::new()
                            .direction(ScrollDirection::Both)
                            .fill()
                            .content(build_scroll_canvas()),
                    )
                    .label("Showcase two-axis scroll area"),
                ),
        )
}

fn build_scroll_canvas() -> Container {
    let theme = current_theme();
    let mut row = Container::new()
        .row()
        .gap(14.0)
        .padding(16.0)
        .size(560.0, 280.0)
        .background(theme.colors.surface_done);

    for column in 0..4 {
        let accent = if column % 2 == 0 {
            theme.colors.primary.with_alpha(0.22)
        } else {
            theme.colors.primary_hovered.with_alpha(0.18)
        };
        row = row.child(
            Container::new()
                .column()
                .gap(12.0)
                .width(116.0)
                .child(sample_tile(&format!("Lane {}", column + 1), accent, 72.0))
                .child(sample_tile(
                    if column % 2 == 0 { "Scroll" } else { "Canvas" },
                    theme.colors.surface_variant,
                    112.0,
                ))
                .child(sample_tile("Viewport", theme.colors.surface, 56.0)),
        );
    }

    row
}

fn sample_tile(label: &str, color: Color, height: f32) -> Container {
    let theme = current_theme();
    Container::new()
        .height(height)
        .padding(14.0)
        .background(color)
        .border(1.0, theme.colors.border)
        .corner_radius(10.0)
        .child(Text::new(label).size(14.0).bold())
}

fn build_virtual_list_sample() -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(10.0)
        .child(Text::new("Virtualized list").size(14.0).bold())
        .child(
            Container::new()
                .height(220.0)
                .background(theme.colors.surface_variant)
                .border(1.0, theme.colors.border)
                .corner_radius(12.0)
                .child(
                    Semantics::new(
                        List::virtualized(240, 38.0, |index| {
                            let theme = current_theme();
                            Box::new(
                                Container::new()
                                    .fill_width()
                                    .height(38.0)
                                    .padding(10.0)
                                    .background(if index % 2 == 0 {
                                        theme.colors.surface
                                    } else {
                                        theme.colors.surface_done
                                    })
                                    .border(1.0, theme.colors.border)
                                    .corner_radius(8.0)
                                    .child(
                                        Text::new(format!("Row {}", index + 1))
                                            .size(13.0)
                                            .color(theme.colors.text_primary),
                                    ),
                            )
                        })
                        .overscan(4)
                        .vertical()
                        .fill(),
                    )
                    .label("Showcase virtualized list"),
                ),
        )
}

fn rendering_atlas_card() -> Container {
    let theme = current_theme();
    section_card(
        "Rendering atlas",
        "Three small diagnostics in one frame: pixel alignment, stroke + clip, and text balance.",
        Container::new().column().child(
            Container::new()
                .padding(12.0)
                .background(Color::from_hex(0x0B0E13))
                .border(1.0, theme.colors.border)
                .corner_radius(12.0)
                .child(RenderingAtlas::new()),
        ),
    )
}

struct RenderingAtlas {
    id: WidgetId,
}

impl RenderingAtlas {
    fn new() -> Self {
        Self {
            id: WidgetId::default(),
        }
    }
}

impl Widget for RenderingAtlas {
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
                height: length(320.0),
            },
            min_size: Size {
                width: percent(1.0),
                height: length(320.0),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        rendering_atlas_scene(ctx);
    }
}

fn scene_label_style(size: f32, color: Color) -> TextStyle {
    TextStyle::default()
        .with_family(current_theme().typography.font_family)
        .with_size(size)
        .with_color(color)
}

fn rendering_atlas_scene(ctx: &mut PaintContext) {
    let bounds = ctx.bounds().inset(8.0);
    let gap = 12.0;
    let panel_width = (bounds.width - gap * 2.0) / 3.0;
    let panel_height = bounds.height;

    let left = Rect::new(bounds.x, bounds.y, panel_width, panel_height);
    let middle = Rect::new(
        bounds.x + panel_width + gap,
        bounds.y,
        panel_width,
        panel_height,
    );
    let right = Rect::new(
        bounds.x + (panel_width + gap) * 2.0,
        bounds.y,
        panel_width,
        panel_height,
    );

    pixel_alignment_scene(ctx, left);
    stroke_and_clip_scene(ctx, middle);
    text_rendering_scene(ctx, right);
}

fn pixel_alignment_scene(ctx: &mut PaintContext, bounds: Rect) {
    let theme = current_theme();
    let bounds = bounds.inset(2.0);
    let text = scene_label_style(11.0, theme.colors.text_muted);
    let bright = Color::WHITE.with_alpha(0.95);
    let dim = Color::from_hex(0x090B0F).with_alpha(0.95);

    ctx.fill_bordered_rect(
        bounds,
        Color::from_hex(0x080A0F),
        14.0,
        1.0,
        theme.colors.border,
    );

    let vertical_origin = Vec2::new(bounds.x + 18.0, bounds.y + 36.0);
    let horizontal_origin = Vec2::new(bounds.x + bounds.width * 0.58, bounds.y + 36.0);
    ctx.draw_text("Pixel alignment", &text, bounds.x + 16.0, bounds.y + 10.0);

    for index in 0..32 {
        let x = vertical_origin.x + index as f32 * 3.0;
        let color = if index % 2 == 0 { bright } else { dim };
        ctx.stroke_line(
            Vec2::new(x, vertical_origin.y),
            Vec2::new(x, vertical_origin.y + 92.0),
            1.0,
            color,
        );
    }

    for index in 0..32 {
        let y = horizontal_origin.y + index as f32 * 3.0;
        let color = if index % 2 == 0 { bright } else { dim };
        ctx.stroke_line(
            Vec2::new(horizontal_origin.x, y),
            Vec2::new(horizontal_origin.x + 96.0, y),
            1.0,
            color,
        );
    }

    let ramp_origin = Vec2::new(bounds.x + 20.0, bounds.y + bounds.height - 70.0);
    for index in 0..9 {
        let size = (index + 1) as f32;
        let x = ramp_origin.x + index as f32 * 18.0;
        let y = ramp_origin.y + (9 - index) as f32 * 1.5;
        ctx.fill_rect(Rect::new(x, y, size, size), bright);
    }
}

fn stroke_and_clip_scene(ctx: &mut PaintContext, bounds: Rect) {
    let theme = current_theme();
    let bounds = bounds.inset(2.0);
    let text = scene_label_style(11.0, theme.colors.text_muted);
    let ink = Color::WHITE.with_alpha(0.92);

    ctx.fill_bordered_rect(
        bounds,
        Color::from_hex(0x080A0F),
        14.0,
        1.0,
        theme.colors.border,
    );
    ctx.draw_text("Stroke + clip", &text, bounds.x + 16.0, bounds.y + 10.0);

    let line_left = bounds.x + 18.0;
    for (index, width) in [1.0, 2.0, 3.0, 4.0].into_iter().enumerate() {
        let y = bounds.y + 54.0 + index as f32 * 24.0;
        ctx.draw_text(&format!("{} px", width as i32), &text, line_left, y - 14.0);
        ctx.stroke_line(
            Vec2::new(line_left + 46.0, y),
            Vec2::new(bounds.x + bounds.width - 20.0, y),
            width,
            ink,
        );
    }

    let rect_origin_x = bounds.x + 18.0;
    for (index, border) in [1.0, 2.0, 3.0].into_iter().enumerate() {
        let x = rect_origin_x + index as f32 * 68.0;
        ctx.fill_bordered_rect(
            Rect::new(x, bounds.y + 160.0, 52.0, 52.0),
            Color::from_hex(0x10151E),
            10.0,
            border,
            Color::WHITE.with_alpha(0.84),
        );
        ctx.draw_text(&format!("{} px", border as i32), &text, x, bounds.y + 220.0);
    }

    let clip = Rect::new(
        bounds.x + 18.0,
        bounds.y + bounds.height - 92.0,
        bounds.width - 36.0,
        58.0,
    );
    ctx.fill_bordered_rect(
        clip,
        Color::from_hex(0x0E131B),
        10.0,
        1.0,
        theme.colors.border,
    );

    ctx.push_clip(clip);
    ctx.push_translation((clip.x + 10.0, clip.y + 10.0));
    for index in 0..12 {
        let x = index as f32 * 32.0 - 24.0;
        let color = if index % 2 == 0 {
            theme.colors.primary.with_alpha(0.9)
        } else {
            Color::WHITE.with_alpha(0.78)
        };
        ctx.fill_rect(Rect::new(x, 0.0, 18.0, 34.0), color);
        ctx.stroke_line(
            Vec2::new(x, 36.0),
            Vec2::new(x + 18.0, 10.0),
            1.0,
            Color::from_hex(0x111827),
        );
    }
    ctx.pop_translation();
    ctx.pop_clip();
}

fn text_rendering_scene(ctx: &mut PaintContext, bounds: Rect) {
    let theme = current_theme();
    let bounds = bounds.inset(2.0);
    let gutter = 10.0;
    let swatch_width = bounds.width - 36.0;
    let swatch_height = (bounds.height - 62.0 - gutter) / 2.0;
    let dark = Rect::new(
        bounds.x + 18.0,
        bounds.y + 40.0,
        swatch_width,
        swatch_height,
    );
    let light = Rect::new(
        dark.x,
        dark.y + dark.height + gutter,
        swatch_width,
        swatch_height,
    );
    let label = scene_label_style(11.0, theme.colors.text_muted);
    let sentence = "Text stays crisp.";

    ctx.fill_bordered_rect(
        bounds,
        Color::from_hex(0x080A0F),
        14.0,
        1.0,
        theme.colors.border,
    );
    ctx.draw_text("Text rendering", &label, bounds.x + 16.0, bounds.y + 10.0);
    ctx.fill_bordered_rect(
        dark,
        Color::from_hex(0x10141A),
        12.0,
        1.0,
        theme.colors.border,
    );
    ctx.fill_bordered_rect(
        light,
        Color::from_hex(0xF5F7FA),
        12.0,
        1.0,
        Color::from_hex(0xB9C0CB),
    );
    ctx.draw_text("Dark swatch", &label, dark.x + 14.0, dark.y + 10.0);
    ctx.draw_text("Light swatch", &label, light.x + 14.0, light.y + 10.0);

    let sizes = [12.0, 16.0, 22.0];
    let alphas = [0.42, 0.72, 1.0];
    ctx.push_clip(dark);
    for index in 0..sizes.len() {
        let y = dark.y + 36.0 + index as f32 * 30.0;
        let dark_style = scene_label_style(sizes[index], Color::WHITE.with_alpha(alphas[index]));
        ctx.draw_text(sentence, &dark_style, dark.x + 14.0, y);
    }
    ctx.pop_clip();

    ctx.push_clip(light);
    for index in 0..sizes.len() {
        let y = light.y + 36.0 + index as f32 * 30.0;
        let light_style = scene_label_style(
            sizes[index],
            Color::from_hex(0x111827).with_alpha(alphas[index]),
        );
        ctx.draw_text(sentence, &light_style, light.x + 14.0, y);
    }
    ctx.pop_clip();
}
