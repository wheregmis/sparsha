use sparsh::prelude::*;

#[test]
fn readme_example_builds_against_the_frozen_surface() {
    let _example: fn() -> Result<(), sparsh::AppRunError> = || {
        let _app = App::new()
            .title("Hello Sparsh")
            .size(960, 640)
            .theme(Theme::light())
            .router(
                Router::new()
                    .route("/", || {
                        Box::new(
                            Container::new()
                                .fill()
                                .center()
                                .gap(16.0)
                                .child(Text::new("Build UI with a GPU-first stack."))
                                .child(Button::new("Click me"))
                                .child(TextInput::new().placeholder("Type here...")),
                        )
                    })
                    .fallback("/"),
            );
        Ok(())
    };
}
