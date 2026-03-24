use sparsha::prelude::*;

#[test]
fn bon_component_surface_builds_from_the_public_crate_root() {
    let runtime = sparsha::signals::RuntimeHandle::new();
    runtime.run_with_current(|| {
        let mut host = Provider::new(
            String::from("outer"),
            component()
                .render(|cx: &mut ComponentContext<'_>| {
                    let count = cx.signal(1usize);
                    let label = cx.use_context_or_else(|| String::from("missing"));
                    Text::builder()
                        .content(format!("{label}: {}", count.get()))
                        .build()
                })
                .call(),
        );

        let mut build = BuildContext::default();
        fn rebuild(widget: &mut dyn Widget, build: &mut BuildContext, path: &mut Vec<usize>) {
            build.set_path(path);
            widget.rebuild(build);
            widget.enter_build_scope(build);
            let child_keys: Vec<_> = (0..widget.children().len())
                .map(|index| widget.child_path_key(index))
                .collect();
            for (index, child) in widget.children_mut().iter_mut().enumerate() {
                path.push(child_keys[index]);
                rebuild(child.as_mut(), build, path);
                path.pop();
            }
            widget.exit_build_scope(build);
        }
        let mut path = Vec::new();
        rebuild(&mut host, &mut build, &mut path);

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
            .gap(16.0)
            .child(Center::new(Padding::all(
                16.0,
                Container::column()
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(12.0)
                    .child(
                        Text::builder()
                            .content("hello")
                            .bold(true)
                            .line_height(1.4)
                            .fill_width(true)
                            .wrap(TextWrap::Word)
                            .break_mode(TextBreakMode::BreakWord)
                            .align(TextAlign::Center)
                            .overflow(TextOverflow::Ellipsis)
                            .build(),
                    )
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
                        Expanded::new(
                            TextArea::builder()
                                .value("notes")
                                .fill_width(true)
                                .on_change(|_| {})
                                .build(),
                        )
                        .flex(1.0),
                    )
                    .child(
                        Scroll::vertical(
                            List::virtualized_builder()
                                .item_count(20)
                                .item_extent(28.0)
                                .item_builder(|index| {
                                    Box::new(
                                        Text::builder().content(format!("row {index}")).build(),
                                    )
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
                    ),
            )))
            .child(
                SizedBox::new().size(180.0, 120.0).child(
                    Stack::new()
                        .aligned(
                            Alignment::Center,
                            Text::builder()
                                .content("overlay")
                                .fill_width(true)
                                .align(TextAlign::Center)
                                .overflow(TextOverflow::Clip)
                                .build(),
                        )
                        .positioned(
                            Positioned::new(Button::builder().label("+").on_click(|| {}).build())
                                .right(8.0)
                                .bottom(8.0),
                        ),
                ),
            );
    });
}

#[test]
fn provider_surface_compiles_from_the_public_crate_root() {
    let runtime = sparsha::signals::RuntimeHandle::new();
    runtime.run_with_current(|| {
        let _tree = Provider::new(
            String::from("outer"),
            Provider::new(
                String::from("inner"),
                component()
                    .render(|cx| {
                        let label = cx.use_context_or(String::from("missing"));
                        Text::builder().content(label).build()
                    })
                    .call(),
            ),
        );
    });
}
