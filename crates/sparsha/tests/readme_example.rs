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
                        component()
                            .render(|cx| {
                                let task = cx.use_task("readme.example", "echo");
                                let _ = task.pending();
                                Container::column()
                                    .fill()
                                    .center()
                                    .gap(16.0)
                                    .child(
                                        Text::builder()
                                            .content("Build UI with a GPU-first stack.")
                                            .build(),
                                    )
                                    .child(Button::builder().label("Click me").build())
                                    .child(TextInput::builder().placeholder("Type here...").build())
                            })
                            .call()
                    })])
                    .fallback("/")
                    .build(),
            );
        Ok(())
    };
}
