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
fn shipped_surface_no_longer_mentions_component_builder() {
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
    }
}

#[test]
fn shipped_surface_documents_the_bon_component_path() {
    let readme = read_repo_file("README.md");
    let todo = read_repo_file("examples/todo/src/main.rs");
    let showcase = read_repo_file("examples/showcase/src/main.rs");
    let component_module = read_repo_file("crates/sparsha/src/component.rs");

    assert!(readme.contains("component().render(...).call()"));
    assert!(todo.contains("component().render("));
    assert!(showcase.contains("component()\n"));
    assert!(component_module.contains("#[builder]"));
}
