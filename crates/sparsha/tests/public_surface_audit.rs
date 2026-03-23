use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn read_repo_file(path: &str) -> String {
    fs::read_to_string(repo_root().join(path)).expect(path)
}

#[test]
fn shipped_surface_no_longer_mentions_removed_authoring_entrypoints() {
    let audited_files = [
        "README.md",
        "docs/api-surface.md",
        "examples/README.md",
        "examples/showcase/src/main.rs",
        "examples/todo/src/main.rs",
        "crates/sparsha/src/component.rs",
        "crates/sparsha/src/lib.rs",
    ];

    for path in audited_files {
        let contents = read_repo_file(path);
        assert!(
            !contents.contains("component_builder"),
            "{path} still references component_builder",
        );
        assert!(
            !contents.contains("App::new("),
            "{path} still references App::new"
        );
        assert!(
            !contents.contains("Router::new("),
            "{path} still references Router::new",
        );
    }
}

#[test]
fn shipped_examples_keep_leaf_widgets_on_builder_paths() {
    let audited_files = [
        "examples/kitchen-sink/src/main.rs",
        "examples/showcase/src/main.rs",
        "examples/todo/src/main.rs",
    ];

    for path in audited_files {
        let contents = read_repo_file(path);
        assert!(
            !contents.contains("Text::new("),
            "{path} still uses Text::new for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("Button::new("),
            "{path} still uses Button::new for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("TextInput::new("),
            "{path} still uses TextInput::new for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("TextArea::new("),
            "{path} still uses TextArea::new for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("Checkbox::new("),
            "{path} still uses Checkbox::new for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("Checkbox::with_checked("),
            "{path} still uses Checkbox::with_checked for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("Text::header("),
            "{path} still uses Text::header for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("Text::subheader("),
            "{path} still uses Text::subheader for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains("Text::caption("),
            "{path} still uses Text::caption for example-facing leaf widget composition",
        );
        assert!(
            !contents.contains(".build().on_click("),
            "{path} still chains on_click after build in example-facing composition",
        );
        assert!(
            !contents.contains(".build().on_toggle("),
            "{path} still chains on_toggle after build in example-facing composition",
        );
        assert!(
            !contents.contains(".build().on_change("),
            "{path} still chains on_change after build in example-facing composition",
        );
        assert!(
            !contents.contains(".build().on_submit("),
            "{path} still chains on_submit after build in example-facing composition",
        );
    }
}

#[test]
fn structural_widgets_do_not_reintroduce_parallel_public_builders() {
    let container = read_repo_file("crates/sparsha-widgets/src/container.rs");
    let scroll = read_repo_file("crates/sparsha-widgets/src/scroll.rs");
    let semantics = read_repo_file("crates/sparsha-widgets/src/semantics.rs");
    let list = read_repo_file("crates/sparsha-widgets/src/list.rs");
    let provider = read_repo_file("crates/sparsha-widgets/src/provider.rs");

    assert!(!container.contains("pub fn new("));
    assert!(!container.contains("impl Default for Container {"));
    assert!(!container.contains("start_fn(name = builder"));
    assert!(container.contains("pub fn row() -> Self"));
    assert!(container.contains("pub fn column() -> Self"));
    assert!(container.contains("pub enum MainAxisAlignment"));
    assert!(container.contains("pub enum CrossAxisAlignment"));
    assert!(container.contains("pub fn main_axis_alignment("));
    assert!(container.contains("pub fn cross_axis_alignment("));

    assert!(!scroll.contains("pub fn new("));
    assert!(!scroll.contains("impl Default for Scroll {"));
    assert!(!scroll.contains("start_fn(name = builder"));
    assert!(!scroll.contains("pub fn content("));
    assert!(!scroll.contains("pub fn vertical(mut self)"));
    assert!(!scroll.contains("pub fn horizontal(mut self)"));
    assert!(scroll.contains("pub fn vertical(widget: impl IntoWidget) -> Self"));
    assert!(scroll.contains("pub fn horizontal(widget: impl IntoWidget) -> Self"));
    assert!(scroll.contains("pub fn both(widget: impl IntoWidget) -> Self"));

    assert!(!semantics.contains("start_fn(name = builder"));
    assert!(!list.contains("pub fn new("));
    assert!(!list.contains("fn builder_init("));
    assert!(!list.contains("pub fn with_items("));
    assert!(!list.contains("impl Default for List {"));
    assert!(list.contains("pub fn empty() -> Self"));
    assert!(list.contains("start_fn(name = virtualized_builder"));
    assert!(provider.contains("pub fn new(value: T, child: impl IntoWidget) -> Self"));
    assert!(provider.contains("ctx.push_context(self.value.clone())"));
}

#[test]
fn app_and_router_do_not_reintroduce_legacy_public_constructors() {
    let app = read_repo_file("crates/sparsha/src/app.rs");
    let router = read_repo_file("crates/sparsha/src/router.rs");

    assert!(!app.contains("pub fn new("));
    assert!(!app.contains("impl Default for App {"));
    assert!(app.contains("start_fn(name = builder"));

    assert!(!router.contains("pub fn new("));
    assert!(!router.contains("impl Default for Router {"));
    assert!(router.contains("start_fn(name = builder"));
}

#[test]
fn leaf_widgets_do_not_reintroduce_legacy_public_constructors() {
    let text = read_repo_file("crates/sparsha-widgets/src/text.rs");
    let button = read_repo_file("crates/sparsha-widgets/src/button.rs");
    let checkbox = read_repo_file("crates/sparsha-widgets/src/checkbox.rs");
    let text_input = read_repo_file("crates/sparsha-widgets/src/text_input.rs");
    let text_area = read_repo_file("crates/sparsha-widgets/src/text_area.rs");

    assert!(!text.contains("pub fn new("));
    assert!(!text.contains("pub fn header("));
    assert!(!text.contains("pub fn subheader("));
    assert!(!text.contains("pub fn caption("));
    assert!(!text.contains("pub fn color("));
    assert!(!text.contains("pub fn size("));
    assert!(!text.contains("pub fn bold("));
    assert!(!text.contains("pub fn italic("));
    assert!(!text.contains("pub fn align("));
    assert!(!text.contains("pub fn center("));
    assert!(!text.contains("pub fn right("));
    assert!(!text.contains("pub fn variant("));
    assert!(!button.contains("pub fn new("));
    assert!(!button.contains("pub fn with_style("));
    assert!(!button.contains("pub fn on_click("));
    assert!(!button.contains("pub fn background("));
    assert!(!button.contains("pub fn text_color("));
    assert!(!button.contains("pub fn corner_radius("));
    assert!(!button.contains("pub fn disabled("));
    assert!(!checkbox.contains("pub fn new("));
    assert!(!checkbox.contains("pub fn with_checked("));
    assert!(!checkbox.contains("pub fn with_style("));
    assert!(!checkbox.contains("pub fn on_toggle("));
    assert!(!checkbox.contains("pub fn checked("));
    assert!(!checkbox.contains("pub fn disabled("));
    assert!(!checkbox.contains("pub fn size("));
    assert!(!checkbox.contains("impl Default for Checkbox {"));
    assert!(!text_input.contains("pub fn new("));
    assert!(!text_input.contains("pub fn with_style("));
    assert!(!text_input.contains("pub fn on_change("));
    assert!(!text_input.contains("pub fn on_submit("));
    assert!(!text_input.contains("pub fn value("));
    assert!(!text_input.contains("pub fn placeholder("));
    assert!(!text_input.contains("pub fn fill_width("));
    assert!(!text_input.contains("impl Default for TextInput {"));
    assert!(!text_area.contains("pub fn new("));
    assert!(!text_area.contains("pub fn with_style("));
    assert!(!text_area.contains("pub fn on_change("));
    assert!(!text_area.contains("pub fn value("));
    assert!(!text_area.contains("pub fn placeholder("));
    assert!(!text_area.contains("pub fn fill_width("));
    assert!(!text_area.contains("impl Default for TextArea {"));
}

#[test]
fn shipped_surface_documents_the_bon_authoring_paths() {
    let readme = read_repo_file("README.md");
    let api_surface = read_repo_file("docs/api-surface.md");
    let examples_readme = read_repo_file("examples/README.md");
    let todo = read_repo_file("examples/todo/src/main.rs");
    let showcase = read_repo_file("examples/showcase/src/main.rs");
    let component_module = read_repo_file("crates/sparsha/src/component.rs");
    let widgets_lib = read_repo_file("crates/sparsha-widgets/src/lib.rs");

    assert!(readme.contains("component().render(...).call()"));
    assert!(readme.contains("App::builder()"));
    assert!(readme.contains("Router::builder()"));
    assert!(api_surface.contains("Container::column()"));
    assert!(api_surface.contains("Container::row()"));
    assert!(api_surface.contains("Container::main_axis_alignment(...)"));
    assert!(api_surface.contains("Container::cross_axis_alignment(...)"));
    assert!(api_surface.contains("Scroll::vertical(...)"));
    assert!(api_surface.contains("List::empty()"));
    assert!(api_surface.contains("Provider::new(...)"));
    assert!(api_surface.contains("ComponentContext::use_context::<T>() -> Option<T>"));
    assert!(api_surface.contains("use_context_or(...)"));
    assert!(api_surface.contains("use_context_or_else(...)"));
    assert!(api_surface.contains("viewport()"));
    assert!(api_surface.contains("navigator()"));
    assert!(api_surface.contains("task_runtime()"));
    assert!(api_surface.contains("Semantics::new(...)"));
    assert!(api_surface.contains("List::virtualized_builder()"));
    assert!(api_surface.contains("TextVariant::Header"));
    assert!(todo.contains("component().render("));
    assert!(todo.contains("App::builder()"));
    assert!(showcase.contains("Router::builder()"));
    assert!(showcase.contains("component()\n"));
    assert!(component_module.contains("#[builder]"));
    assert!(component_module.contains("pub fn use_context<T: Clone + 'static>(&self) -> Option<T>"));
    assert!(component_module
        .contains("pub fn use_context_or<T: Clone + 'static>(&self, default: T) -> T"));
    assert!(component_module.contains(
        "pub fn use_context_or_else<T: Clone + 'static>(&self, default: impl FnOnce() -> T) -> T"
    ));
    assert!(component_module.contains("self.build.context::<T>()"));
    assert!(widgets_lib.contains("pub use provider::Provider;"));
    assert!(widgets_lib.contains("CrossAxisAlignment"));
    assert!(widgets_lib.contains("MainAxisAlignment"));
    assert!(readme.contains("Button::builder()"));
    assert!(readme.contains("Provider::new("));
    assert!(readme.contains("main_axis_alignment(MainAxisAlignment::Center)"));
    assert!(readme.contains("cross_axis_alignment(CrossAxisAlignment::Center)"));
    assert!(examples_readme.contains("Provider::new(...)"));
    assert!(examples_readme.contains("cx.use_context::<T>()"));
    assert!(examples_readme.contains("cx.use_context_or(...)"));
    assert!(examples_readme.contains("cx.use_context_or_else(...)"));
    assert!(examples_readme.contains("Container::main_axis_alignment(...)"));
    assert!(examples_readme.contains("Container::cross_axis_alignment(...)"));
    assert!(readme.contains("cx.viewport()"));
    assert!(examples_readme.contains("cx.viewport()"));
}
