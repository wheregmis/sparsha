use sparsha::prelude::*;

fn material_counter_theme() -> Theme {
    let mut theme = Theme::light();
    theme.colors.background = Color::from_hex(0xFAFAFA);
    theme.colors.surface = Color::WHITE;
    theme.colors.surface_variant = Color::from_hex(0xF5F5F5);
    theme.colors.text_primary = Color::from_hex(0x212121);
    theme.colors.text_muted = Color::from_hex(0x757575);
    theme.colors.primary = Color::from_hex(0x2196F3);
    theme.colors.primary_hovered = Color::from_hex(0x1E88E5);
    theme.colors.primary_pressed = Color::from_hex(0x1976D2);
    theme.colors.border = Color::from_hex(0xE0E0E0);
    theme.typography.body_size = 16.0;
    theme.typography.small_size = 14.0;
    theme.typography.title_size = 20.0;
    theme.typography.button_size = 16.0;
    theme.radii.md = 4.0;
    theme.radii.lg = 28.0;
    theme.controls.control_height = 40.0;
    theme.controls.control_padding_x = 16.0;
    theme.controls.control_padding_y = 10.0;
    theme
}

fn floating_action_button_style(theme: &Theme) -> ButtonStyle {
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
        font_size: 32.0,
        min_width: 56.0,
        min_height: 56.0,
    }
}

fn main() -> Result<(), sparsha::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsha::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::builder()
        .title("Sparsha Counter")
        .width(430)
        .height(760)
        .theme(material_counter_theme())
        .router(
            Router::builder()
                .routes(vec![Route::new("/", || {
                    component().render(counter_app).call()
                })])
                .fallback("/")
                .build(),
        )
        .build()
        .run()
}

fn counter_app(cx: &mut ComponentContext<'_>) -> Container {
    let count = cx.signal(0i32);
    let theme = cx.theme();
    let fab_style = floating_action_button_style(&theme);

    Container::column()
        .fill()
        .background(theme.colors.background)
        .child(
            Container::column()
                .fill_width()
                .height(56.0)
                .padding_sides(16.0, 16.0, 0.0, 0.0)
                .main_axis_alignment(MainAxisAlignment::Center)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .background(theme.colors.primary)
                .child(
                    Text::builder()
                        .content("Sparsha Demo Home Page")
                        .font_size(20.0)
                        .color(Color::WHITE)
                        .fill_width(true)
                        .align(TextAlign::Center)
                        .build(),
                ),
        )
        .child(
            Container::column()
                .fill_width()
                .flex_grow(1.0)
                .main_axis_alignment(MainAxisAlignment::Center)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .padding_sides(24.0, 24.0, 24.0, 24.0)
                .gap(12.0)
                .child(
                    Text::builder()
                        .content("You have pushed the button this many times:")
                        .font_size(16.0)
                        .color(theme.colors.text_muted)
                        .fill_width(true)
                        .align(TextAlign::Center)
                        .build(),
                )
                .child(
                    Text::builder()
                        .content(count.get().to_string())
                        .font_size(72.0)
                        .bold(true)
                        .color(theme.colors.text_primary)
                        .fill_width(true)
                        .align(TextAlign::Center)
                        .build(),
                )
        )
        .child(
            Container::row()
                .fill_width()
                .padding_sides(24.0, 24.0, 0.0, 28.0)
                .main_axis_alignment(MainAxisAlignment::End)
                .child(
                    Button::builder()
                        .label("+")
                        .style(fab_style)
                        .on_click(move || {
                            count.set(count.get() + 1);
                        })
                        .build(),
                ),
        )
}
