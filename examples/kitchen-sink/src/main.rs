//! Kitchen Sink - Interactive widget testing

use sparsh::prelude::*;

fn main() -> Result<(), sparsh::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsh::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::new()
        .title("Kitchen Sink - Sparsh")
        .size(1200, 900)
        .background(Color::from_hex(0x0F172A))
        .theme(Theme::light())
        .router(Router::new().route("/", build_ui).fallback("/"))
        .run()
}

fn build_ui() -> Box<dyn Widget> {
    Box::new(
        Container::new()
            .fill()
            .row()
            .background(Color::from_hex(0x0F172A))
            .child(build_sidebar())
            .child(build_main_area()),
    )
}

/// Left sidebar with button gallery and text samples
fn build_sidebar() -> Container {
    Container::new()
        .column()
        .gap(32.0) // Increased gap between major sections
        .padding(24.0)
        .width(250.0) // Increased width
        .fill_height()
        .background(Color::from_hex(0x1E293B))
        .child(
            Text::new("Kitchen Sink")
                .size(24.0)
                .bold()
                .color(Color::WHITE),
        )
        // Section: Button Gallery
        .child(
            Container::new()
                .column()
                .gap(12.0)
                .child(
                    Text::new("Buttons")
                        .size(14.0)
                        .bold()
                        .color(Color::from_hex(0x94A3B8)),
                )
                .child(
                    Button::new("Default")
                        .background(Color::from_hex(0x3B82F6))
                        .on_click(|| {
                            log::info!("Default button clicked!");
                        }),
                )
                .child(
                    Button::new("Success")
                        .background(Color::from_hex(0x22C55E))
                        .on_click(|| {
                            log::info!("Success button clicked!");
                        }),
                )
                .child(
                    Button::new("Danger")
                        .background(Color::from_hex(0xEF4444))
                        .on_click(|| {
                            log::info!("Danger button clicked!");
                        }),
                )
                .child(
                    Button::new("Warning")
                        .background(Color::from_hex(0xF59E0B))
                        .on_click(|| {
                            log::info!("Warning button clicked!");
                        }),
                )
                .child(
                    Button::new("Secondary")
                        .background(Color::from_hex(0x64748B))
                        .on_click(|| {
                            log::info!("Secondary button clicked!");
                        }),
                ),
        )
        // Section: Typography
        .child(
            Container::new()
                .column()
                .gap(16.0)
                .child(
                    Text::new("Typography")
                        .size(14.0)
                        .bold()
                        .color(Color::from_hex(0x94A3B8)),
                )
                .child(
                    Text::new("Heading Text")
                        .size(28.0)
                        .bold()
                        .color(Color::WHITE),
                )
                .child(
                    Text::new("Body text example that wraps\nto fit the sidebar.")
                        .size(16.0)
                        .color(Color::from_hex(0xE2E8F0)),
                )
                .child(
                    Text::new("Small caption text")
                        .size(12.0)
                        .color(Color::from_hex(0x94A3B8)),
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
                .child(build_scroll_section()),
        )
}

/// Input fields section
fn build_input_section() -> Container {
    section(
        "Input Fields",
        Container::new()
            .column()
            .gap(12.0)
            .fill_width()
            .child(TextInput::new().fill_width().placeholder("Enter text..."))
            .child(
                TextInput::new()
                    .fill_width()
                    .placeholder("Email address..."),
            )
            .child(TextInput::new().fill_width().placeholder("Password...")),
    )
}

/// Nested and overlapping containers section
fn build_container_section() -> Container {
    Container::new()
        .column()
        .gap(16.0)
        .padding(24.0)
        .background(Color::from_hex(0x1E293B))
        .corner_radius(12.0)
        .child(
            Text::new("Nested Containers")
                .size(18.0)
                .bold()
                .color(Color::WHITE),
        )
        // 3-level nesting
        .child(
            Container::new()
                .padding(16.0)
                .background(Color::from_hex(0x3B82F6).with_alpha(0.3))
                .corner_radius(8.0)
                .child(
                    Container::new()
                        .padding(16.0)
                        .background(Color::from_hex(0x22C55E).with_alpha(0.3))
                        .corner_radius(8.0)
                        .child(
                            Container::new()
                                .padding(16.0)
                                .background(Color::from_hex(0x8B5CF6).with_alpha(0.3))
                                .corner_radius(8.0)
                                .child(Text::new("Level 3").size(14.0).color(Color::WHITE)),
                        ),
                ),
        )
}

/// Scrollable content section
fn build_scroll_section() -> Container {
    let mut scroll_content = Container::new().column().gap(8.0);

    // Add 20 list items
    for i in 0..20 {
        scroll_content = scroll_content.child(
            Container::new()
                .padding(12.0)
                .min_size(0.0, 40.0)
                .background(if i % 2 == 0 {
                    Color::from_hex(0x334155)
                } else {
                    Color::from_hex(0x1E293B)
                })
                .corner_radius(4.0)
                .child(
                    Text::new(format!("Item {}", i + 1))
                        .size(14.0)
                        .color(Color::WHITE),
                ),
        );
    }

    section(
        "Scrollable Area",
        Container::new()
            .fill_width()
            .height(300.0)
            .child(Scroll::new().vertical().fill().content(scroll_content)),
    )
}

/// Creates a labeled section container
fn section(title: &str, content: Container) -> Container {
    Container::new()
        .column()
        .fill_width()
        .gap(16.0)
        .padding(24.0)
        .background(Color::from_hex(0x1E293B))
        .corner_radius(12.0)
        .child(Text::new(title).size(18.0).bold().color(Color::WHITE))
        .child(content)
}
