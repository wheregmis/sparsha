use sparsha::prelude::*;

#[test]
fn bon_component_surface_builds_from_the_public_crate_root() {
    let runtime = sparsha::signals::RuntimeHandle::new();
    runtime.run_with_current(|| {
        let mut host = component()
            .render(|cx: &mut ComponentContext<'_>| {
                let count = cx.signal(1usize);
                Text::builder()
                    .content(format!("Count: {}", count.get()))
                    .build()
            })
            .call();

        let mut build = BuildContext::default();
        host.rebuild(&mut build);

        assert_eq!(host.children().len(), 1);
    });
}

#[test]
fn bon_app_and_router_builders_compile_from_the_public_crate_root() {
    let runtime = sparsha::signals::RuntimeHandle::new();
    runtime.run_with_current(|| {
        let _app = App::builder()
            .title("Surface Test")
            .width(960)
            .height(640)
            .theme(Theme::light())
            .router(
                Router::builder()
                    .routes(vec![Route::new("/", || {
                        component()
                            .render(|_| Text::builder().content("home").build())
                            .call()
                    })])
                    .fallback("/")
                    .build(),
            )
            .build();
    });
}

#[test]
fn semantic_structural_widget_surface_compiles_from_the_public_crate_root() {
    let runtime = sparsha::signals::RuntimeHandle::new();
    runtime.run_with_current(|| {
        let email_state = Signal::new(String::new());
        let checked_state = Signal::new(false);
        let _tree = Container::column()
            .gap(12.0)
            .padding(16.0)
            .child(Text::builder().content("hello").bold(true).build())
            .child(
                Button::builder()
                    .label("save")
                    .disabled(false)
                    .on_click(|| {})
                    .build(),
            )
            .child(
                Checkbox::builder()
                    .checked(true)
                    .on_toggle(move |next| checked_state.set(next))
                    .build(),
            )
            .child(
                TextInput::builder()
                    .placeholder("type here")
                    .fill_width(true)
                    .on_change(move |value| email_state.set(value.to_owned()))
                    .on_submit(|_| {})
                    .build(),
            )
            .child(
                TextArea::builder()
                    .value("notes")
                    .fill_width(true)
                    .on_change(|_| {})
                    .build(),
            )
            .child(
                Scroll::vertical(
                    List::virtualized_builder()
                        .item_count(20)
                        .item_extent(28.0)
                        .item_builder(|index| {
                            Box::new(Text::builder().content(format!("row {index}")).build())
                        })
                        .direction(ListDirection::Vertical)
                        .build(),
                )
                .height(120.0)
                .direction(ScrollDirection::Vertical),
            )
            .child(
                Semantics::new(Text::builder().content("semantic label").build())
                    .label("Semantic label"),
            );
    });
}
