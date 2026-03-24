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
    theme.radii.lg = 28.0;
    theme.controls.control_height = 40.0;
    theme.controls.control_padding_x = 16.0;
    theme.controls.control_padding_y = 10.0;
    theme
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

fn counter_app(cx: &mut ComponentContext<'_>) -> Scaffold {
    let count = cx.signal(0i32);
    let theme = cx.theme();

    Scaffold::new(Center::new(Padding::all(
        24.0,
        Container::column()
            .fill_width()
            .gap(theme.spacing.md)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(
                Text::builder()
                    .content("You have pushed the button this many times:")
                    .font_size(16.0)
                    .color(theme.colors.text_muted)
                    .fill_width(true)
                    .align(TextAlign::Center)
                    .overflow(TextOverflow::Clip)
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
                    .overflow(TextOverflow::Clip)
                    .build(),
            ),
    )))
    .background(theme.colors.background)
    .app_bar(AppBar::new("Sparsha Demo Home Page").center_title(true))
    .floating_action_button(
        FloatingActionButton::new("+")
            .accessibility_label("Increment counter")
            .on_click(move || {
                count.set(count.get() + 1);
            }),
    )
}
