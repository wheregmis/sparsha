//! Kitchen Sink - Interactive widget testing

use sparsha::core::Rect;
use sparsha::prelude::*;
use sparsha::text::TextStyle;

fn main() -> Result<(), sparsha::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsha::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::builder()
        .title("Kitchen Sink - Sparsha")
        .width(1200)
        .height(900)
        .theme(Theme::dark())
        .router(
            Router::builder()
                .routes(vec![Route::new("/", build_ui)])
                .fallback("/")
                .build(),
        )
        .build()
        .run()
}

fn build_ui() -> Box<dyn Widget> {
    Box::new(
        Container::new()
            .fill()
            .row()
            .child(build_sidebar())
            .child(build_main_area()),
    )
}

/// Left sidebar with button gallery and text samples
fn build_sidebar() -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(32.0)
        .padding(24.0)
        .width(250.0)
        .fill_height()
        .background(theme.colors.surface)
        .border(1.0, theme.colors.border)
        .child(
            Text::builder()
                .content("Kitchen Sink")
                .variant(TextVariant::Header)
                .build(),
        )
        .child(
            Container::new()
                .column()
                .gap(12.0)
                .child(
                    Text::builder()
                        .content("Buttons")
                        .variant(TextVariant::Caption)
                        .build(),
                )
                .child(
                    Button::builder()
                        .label("Primary Action")
                        .on_click(|| {
                            log::info!("Primary action clicked");
                        })
                        .build(),
                )
                .child(
                    Button::builder()
                        .label("Secondary Action")
                        .corner_radius(10.0)
                        .on_click(|| {
                            log::info!("Secondary action clicked");
                        })
                        .build(),
                )
                .child(
                    Button::builder()
                        .label("Disabled State")
                        .disabled(true)
                        .build(),
                ),
        )
        .child(
            Container::new()
                .column()
                .gap(16.0)
                .child(
                    Text::builder()
                        .content("Typography")
                        .variant(TextVariant::Caption)
                        .build(),
                )
                .child(
                    Text::builder()
                        .content("Heading Text")
                        .variant(TextVariant::Header)
                        .build(),
                )
                .child(
                    Text::builder()
                        .content("Body text example that wraps\nto fit the sidebar.")
                        .build(),
                )
                .child(
                    Text::builder()
                        .content("Small caption text")
                        .variant(TextVariant::Caption)
                        .build(),
                ),
        )
}

/// Main content area with scrollable sections
fn build_main_area() -> Scroll {
    Scroll::new()
        .vertical()
        .flex_grow(1.0)
        .fill_height()
        .content(
            Container::new()
                .column()
                .gap(32.0)
                .padding(32.0)
                .fill_width()
                .child(build_input_section())
                .child(build_container_section())
                .child(build_animation_section())
                .child(build_scroll_section()),
        )
}

/// Input fields section
fn build_input_section() -> Container {
    let theme = current_theme();
    section(
        "Input, Focus, And Editing",
        Container::new()
            .column()
            .gap(12.0)
            .fill_width()
            .child(
                Text::builder()
                    .content(
                        "Use Tab and Shift+Tab to move through the checkbox, single-line fields, and multiline editor. Native and web now share copy, cut, paste, undo, redo, word movement, and IME composition behavior.",
                    )
                    .font_size(13.0)
                    .color(theme.colors.text_muted)
                    .build(),
            )
            .child(
                Container::new()
                    .row()
                    .gap(12.0)
                    .align_start()
                    .child(
                        Semantics::new(Checkbox::builder().checked(true).build())
                            .label("Focusable checkbox in the same tab order"),
                    )
                    .child(
                        Text::builder()
                            .content("Focusable checkbox in the same tab order")
                            .build(),
                    ),
            )
            .child(
                TextInput::builder()
                    .fill_width(true)
                    .placeholder("Single-line input with clipboard + undo")
                    .build(),
            )
            .child(
                TextInput::builder()
                    .fill_width(true)
                    .placeholder("Email address...")
                    .build(),
            )
            .child(
                TextArea::builder()
                    .fill_width(true)
                    .placeholder(
                        "Multiline notes...\nTry Enter, arrow keys, word movement, and paste.",
                    )
                    .build(),
            ),
    )
}

/// Nested and overlapping containers section
fn build_container_section() -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(16.0)
        .padding(24.0)
        .background(theme.colors.surface)
        .border(1.0, theme.colors.border)
        .corner_radius(12.0)
        .child(
            Text::builder()
                .content("Nested Containers")
                .font_size(18.0)
                .bold(true)
                .build(),
        )
        .child(
            Container::new()
                .padding(16.0)
                .background(theme.colors.primary.with_alpha(0.18))
                .corner_radius(8.0)
                .child(
                    Container::new()
                        .padding(16.0)
                        .background(theme.colors.surface_variant.with_alpha(0.9))
                        .corner_radius(8.0)
                        .child(
                            Container::new()
                                .padding(16.0)
                                .background(theme.colors.primary_hovered.with_alpha(0.35))
                                .corner_radius(8.0)
                                .child(
                                    Text::builder()
                                        .content("Level 3")
                                        .font_size(14.0)
                                        .color(Color::WHITE)
                                        .build(),
                                ),
                        ),
                ),
        )
}

fn build_scroll_section() -> Container {
    let theme = current_theme();
    let mut scroll_content = Container::new().column().gap(8.0);

    for i in 0..20 {
        scroll_content = scroll_content.child(
            Container::new()
                .padding(12.0)
                .min_size(0.0, 40.0)
                .background(if i % 2 == 0 {
                    theme.colors.surface_variant
                } else {
                    theme.colors.surface
                })
                .corner_radius(4.0)
                .border(1.0, theme.colors.border)
                .child(
                    Text::builder()
                        .content(format!("Item {}", i + 1))
                        .font_size(14.0)
                        .build(),
                ),
        );
    }

    section(
        "Scrolling And Lists",
        Container::new()
            .column()
            .gap(16.0)
            .fill_width()
            .child(
                Text::builder()
                    .content(
                        "The left demo is a regular two-axis scroll container. The right demo is a fixed-row virtualized list that only realizes the visible range.",
                    )
                    .font_size(13.0)
                    .color(theme.colors.text_muted)
                    .build(),
            )
            .child(
                Container::new()
                    .row()
                    .gap(16.0)
                    .fill_width()
                    .child(
                        Container::new()
                            .flex_grow(1.0)
                            .min_size(0.0, 260.0)
                            .height(260.0)
                            .child(
                                Semantics::new(
                                    Scroll::new()
                                        .direction(ScrollDirection::Both)
                                        .fill()
                                        .content(
                                            Container::new()
                                                .size(720.0, 420.0)
                                                .padding(16.0)
                                                .background(theme.colors.surface)
                                                .border(1.0, theme.colors.border)
                                                .child(scroll_content),
                                        ),
                                )
                                .label("Kitchen sink two-axis scroll area"),
                            ),
                    )
                    .child(
                        Container::new()
                            .flex_grow(1.0)
                            .min_size(0.0, 260.0)
                            .height(260.0)
                            .child(
                                Semantics::new(
                                    List::virtualized_builder()
                                        .item_count(500)
                                        .item_extent(44.0)
                                        .item_builder(|index| {
                                            let theme = current_theme();
                                            Box::new(
                                                Container::new()
                                                    .fill_width()
                                                    .min_size(0.0, 44.0)
                                                    .padding(12.0)
                                                    .background(if index % 2 == 0 {
                                                        theme.colors.surface
                                                    } else {
                                                        theme.colors.surface_variant
                                                    })
                                                    .border(1.0, theme.colors.border)
                                                    .corner_radius(8.0)
                                                    .child(
                                                        Text::builder()
                                                            .content(format!(
                                                                "Virtual row {}",
                                                                index + 1
                                                            ))
                                                            .build(),
                                                    ),
                                            )
                                        })
                                        .overscan(3)
                                        .direction(ListDirection::Vertical)
                                        .fill(true)
                                        .build(),
                                )
                                .label("Kitchen sink virtualized list"),
                            ),
                    ),
            ),
    )
}

fn build_animation_section() -> Container {
    section(
        "Animations",
        Container::new()
            .column()
            .gap(16.0)
            .fill_width()
            .child(
                Text::builder()
                    .content(
                        "Implicit animation: route-card fade uses ImplicitAnimation.\n\
                     Explicit animation: draw surface updates using elapsed_time.\n\
                     Page transitions: router cross-fade overlay between routes.",
                    )
                    .font_size(13.0)
                    .color(current_theme().colors.text_muted)
                    .build(),
            )
            .child(
                DrawSurface::new(|ctx| {
                    ctx.request_next_frame();
                    let bounds = ctx.bounds;
                    let t = (ctx.elapsed_time * 1.2).sin() * 0.5 + 0.5;
                    let bg = lerp_color(
                        Color::from_hex(0x1F2937).with_alpha(0.9),
                        Color::from_hex(0x2563EB).with_alpha(0.9),
                        t,
                    );
                    ctx.fill_bordered_rect(
                        bounds,
                        bg,
                        14.0,
                        1.0,
                        current_theme().colors.border.with_alpha(0.9),
                    );

                    let wave_width = bounds.width * 0.28;
                    let x = bounds.x + (bounds.width - wave_width) * t;
                    ctx.fill_rect(
                        Rect::new(x, bounds.y + 8.0, wave_width, bounds.height - 16.0),
                        current_theme().colors.primary.with_alpha(0.22),
                    );

                    let style = TextStyle::new()
                        .with_family("Inter")
                        .with_size(14.0)
                        .with_color(current_theme().colors.text_primary)
                        .bold();
                    ctx.draw_text(
                        "Explicit animation: DrawSurface timeline",
                        &style,
                        bounds.x + 16.0,
                        bounds.y + 18.0,
                    );
                })
                .fill_width()
                .height(110.0),
            ),
    )
}

fn section(title: &str, content: impl Widget + 'static) -> Container {
    let theme = current_theme();
    Container::new()
        .column()
        .gap(16.0)
        .padding(24.0)
        .background(theme.colors.surface)
        .border(1.0, theme.colors.border)
        .corner_radius(16.0)
        .child(
            Text::builder()
                .content(title)
                .font_size(18.0)
                .bold(true)
                .build(),
        )
        .child(content)
}
