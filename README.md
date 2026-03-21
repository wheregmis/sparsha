# Sparsh

> GPU-first, cross-platform UI for Rust.

Sparsh is a Rust UI framework built around a single widget tree that runs on desktop and on the web. The public 1.0 contract is the curated crate-root surface in `sparsh` and the companion crates, not every reachable implementation module path.

## Status

- Current milestone: Foundation freeze
- Public 1.0 surface: app runner, router, themes, task runtime, widgets, accessibility metadata, signals, input, layout, render, text, and core primitives exposed at crate roots
- Provisional/internal: raw implementation modules, platform glue like `ui_events_winit`, and runtime adapters behind the public accessibility surface

See [docs/api-surface.md](docs/api-surface.md) for the crate-by-crate API inventory.

## Supported Platforms

- Desktop: macOS, Linux, Windows
- Web: `wasm32-unknown-unknown` with retained DOM rendering by default

## Widget Set

The built-in widget layer currently supports:

- `Container`
- `Button`
- `Checkbox`
- `Text`
- `TextInput`
- `TextArea`
- `List`
- `Scroll`
- `DrawSurface`
- `Semantics`

## Web Story

- Default web path: retained DOM rendering driven by the same widget tree as native
- Hybrid path: `DrawSurface` embeds GPU-heavy scenes into an otherwise DOM-backed UI
- Runtime model: DOM rendering stays responsive while background work uses a worker-backed task runtime
- Repo-owned web workflow: root build/serve/smoke scripts wrap the checked-in example `index.html`, `Trunk.toml`, and `sparsh-worker.js` assets

## Current Limitations

- Router paths are static-only; dynamic route patterns are not supported
- CI and release automation are intentionally out of scope for this milestone; verification is local and documented below

## Quick Start

```rust
use sparsh::prelude::*;

fn main() -> Result<(), sparsh::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsh::init_web()?;

    App::new()
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
        )
        .run()
}
```

## Crates

| Crate | Role |
|---|---|
| `sparsh` | App runner, router, task runtime, and public facade |
| `sparsh-core` | Core primitives and GPU bootstrap helpers |
| `sparsh-render` | Draw list, shape pass, text pass, batching |
| `sparsh-layout` | Flexbox layout via `taffy` |
| `sparsh-text` | Font loading, shaping, glyph atlas management |
| `sparsh-input` | Input events, focus management, hit testing |
| `sparsh-signals` | Reactive signal runtime |
| `sparsh-widgets` | Built-in widgets, theme types, paint/build contexts |

## Examples

Run the native examples:

```bash
cargo run -p kitchen-sink
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay
cargo run -p todo
```

Build and serve a web example from the repo root:

```bash
rustup target add wasm32-unknown-unknown
./scripts/web-build-example.sh kitchen-sink
./scripts/web-serve-dist.sh kitchen-sink 4173
```

Run the browser smoke suite:

```bash
npm install
npm run web:install
./scripts/web-smoke.sh
```

Direct per-example `trunk serve` still works for local iteration, but the root scripts are the canonical checked-in build and verification path. More detail lives in [examples/README.md](examples/README.md).

## Verification

Local verification entrypoints:

```bash
./scripts/verify-foundation.sh
./scripts/web-build-all.sh
./scripts/web-smoke.sh
```

`verify-foundation.sh` runs:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo check -p kitchen-sink -p fractal-clock -p hybrid-overlay -p todo --target wasm32-unknown-unknown`

`web-smoke.sh` builds the checked-in web examples, serves the generated `dist/` output for `kitchen-sink`, `hybrid-overlay`, and `todo`, then runs the Playwright smoke suite against those pages.

## More

- [docs/api-surface.md](docs/api-surface.md)
- [examples/README.md](examples/README.md)
- [crates/sparsh/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/lib.rs)

## License

MIT
