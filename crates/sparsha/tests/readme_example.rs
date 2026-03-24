use sparsha::prelude::*;

#[test]
fn readme_example_builds_against_the_frozen_surface() {
    let _example: fn() -> Result<(), sparsha::AppRunError> = || {
        let _app = App::builder()
            .title("Hello Sparsha")
            .width(960)
            .height(640)
            .theme(Theme::light())
            .router(
                Router::builder()
                .routes(vec![Route::new("/", || {
                    Provider::new(
                        ThemeMode::Light,
                        component()
                            .render(|cx| {
                                let task = cx.use_task("readme.example", "echo");
                                let _ = task.pending();
                                let mode = cx.use_context_or(ThemeMode::Light);
                                Container::column()
                                    .fill()
                                    .main_axis_alignment(MainAxisAlignment::Center)
                                    .cross_axis_alignment(CrossAxisAlignment::Center)
                                    .gap(16.0)
                                    .child(
                                        Text::builder()
                                            .content(format!("Build UI with a GPU-first stack. Mode: {mode:?}"))
                                            .build(),
                                    )
                                    .child(Button::builder().label("Click me").build())
                                    .child(TextInput::builder().placeholder("Type here...").build())
                            })
                            .call(),
                    )
                })])
                .fallback("/")
                .build(),
            );
        Ok(())
    };
}

#[test]
fn context_example_builds_against_the_frozen_surface() {
    let _example: fn() -> Result<(), sparsha::AppRunError> = || {
        let _tree = Provider::new(
            ThemeMode::Dark,
            component()
                .render(|cx| {
                    let mode = cx.use_context_or(ThemeMode::Light);
                    Container::column()
                        .fill()
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(16.0)
                        .child(
                            Text::builder()
                                .content(format!("Mode: {mode:?}"))
                                .fill_width(true)
                                .align(TextAlign::Center)
                                .build(),
                        )
                        .child(Button::builder().label("Click me").build())
                })
                .call(),
        );
        Ok(())
    };
}
