use sparsha::prelude::*;

#[test]
fn bon_component_surface_builds_from_the_public_crate_root() {
    let runtime = sparsha::signals::RuntimeHandle::new();
    runtime.run_with_current(|| {
        let mut host = component()
            .render(|cx: &mut ComponentContext<'_>| {
                let count = cx.signal(1usize);
                Text::new(format!("Count: {}", count.get()))
            })
            .call();

        let mut build = BuildContext::default();
        host.rebuild(&mut build);

        assert_eq!(host.children().len(), 1);
    });
}
