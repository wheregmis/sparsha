# Sparsha

> GPU-first, cross-platform UI for Rust.

Sparsha is a Rust UI framework built around a single widget tree that runs on desktop and on the web. The public 1.0 contract is the curated crate-root surface in `sparsha` and the companion crates, not every reachable implementation module path.

## Status

- Current milestone: Release engineering and quality gates
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
- `Provider`
- `Center`, `Padding`, `Expanded`, `Align`, `SizedBox`, `Spacer`
- `Stack`, `Positioned`
- `DrawSurface`
- `Semantics`

Notable current behavior:

- `Scroll` supports vertical, horizontal, and both-axis scrolling with interactive scrollbars
- `List` supports both simple owned-children mode and fixed-extent virtualization for large data sets
- Default widget sizing and focus-ring behavior are aligned through shared theme control tokens
- Semantic layout helpers cover common structure without dropping to raw flex settings: `Center`, `Padding`, `Expanded`, `Stack`, `Positioned`, `Align`, `SizedBox`, and `Spacer`
- Paragraph text layout stays on the `Text` builder surface through `line_height(...)`, `fill_width(...)`, `wrap(TextWrap::Word)`, `max_lines(...)`, and overflow policies such as `TextOverflow::Clip` and `TextOverflow::Ellipsis`
- Normal app screens can be authored as bon-backed function components via `component().render(...).call()` and `ComponentContext`
- Subtree-scoped typed values can be provided with `Provider::new(...)` and read in components via `cx.use_context::<T>()`, `cx.use_context_or(...)`, or `cx.use_context_or_else(...)`
- Built-in framework resources stay on dedicated component accessors such as `cx.viewport()`, `cx.navigator()`, and `cx.task_runtime()`

## Web Story

- Default web path: retained DOM rendering driven by the same widget tree as native
- Hybrid path: `DrawSurface` embeds GPU-heavy scenes into an otherwise DOM-backed UI
- Runtime model: DOM rendering stays responsive while background work uses a worker-backed task runtime
- Repo-owned web workflow: root build/serve/smoke scripts wrap the checked-in example `index.html`, `Trunk.toml`, and `sparsha-worker.js` assets
- Browser wasm tests are checked in as `./scripts/wasm-browser-tests.sh` for the `sparsha` crate's `wasm_bindgen_test` entrypoints

## Task Runtime

- `TaskRuntime` is part of the supported 1.0 crate-root surface
- The currently supported built-in task kinds are `echo`, `sleep_echo`, and `analyze_text`
- The supported contract covers `spawn`, `spawn_keyed`, `cancel`, and result delivery across native and web
- Milestone 6 does not add custom task registration; unknown task kinds should be treated as unsupported

## Current Limitations

- Router paths are static-only; dynamic route patterns are not supported
- Accessibility still needs manual screen-reader verification before 1.0 sign-off
- Final native/web parity sign-off still includes manual smoke checks in addition to automation

## Quick Start

```rust
use sparsha::prelude::*;

fn main() -> Result<(), sparsha::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsha::init_web()?;

    App::builder()
        .title("Hello Sparsha")
        .width(960)
        .height(640)
        .theme(Theme::light())
        .router(
            Router::builder()
                .routes(vec![Route::new("/", || {
                    Container::column()
                        .fill()
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(16.0)
                        .child(
                            Text::builder()
                                .content("Build UI with a GPU-first stack.")
                                .fill_width(true)
                                .align(TextAlign::Center)
                                .overflow(TextOverflow::Ellipsis)
                                .build(),
                        )
                        .child(Button::builder().label("Click me").build())
                        .child(
                            TextInput::builder()
                                .placeholder("Type here...")
                                .build(),
                        )
                })])
                .fallback("/")
                .build(),
        )
        .build()
        .run()
}
```

## Crates

| Crate | Role |
|---|---|
| `sparsha` | App runner, router, task runtime, and public facade |
| `sparsha-core` | Core primitives and GPU bootstrap helpers |
| `sparsha-render` | Draw list, shape pass, text pass, batching |
| `sparsha-layout` | Flexbox layout via `taffy` |
| `sparsha-text` | Font loading, shaping, glyph atlas management |
| `sparsha-input` | Input events, focus management, hit testing |
| `sparsha-signals` | Reactive signal runtime |
| `sparsha-widgets` | Built-in widgets, theme types, paint/build contexts |

## Examples

Run the native examples:

```bash
cargo run -p counter
cargo run -p layout-probe
cargo run -p kitchen-sink
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay
cargo run -p showcase
cargo run -p todo
```

Run an example on mobile with `cargo-mobile2`:

```bash
# one-time tool install
cargo install --git https://github.com/tauri-apps/cargo-mobile2

# Android
./scripts/mobile-run-example.sh kitchen-sink android run

# iOS (macOS only)
./scripts/mobile-run-example.sh kitchen-sink ios run
```

The helper script auto-runs `cargo mobile init --non-interactive` inside the selected example when the mobile project files do not exist yet.

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

Run the lightweight web perf/startup smoke:

```bash
./scripts/web-perf-smoke.sh
```

Run the full local release-readiness suite:

```bash
./scripts/release-readiness.sh
```

Direct per-example `trunk serve` still works for local iteration, but the root scripts are the canonical checked-in build and verification path. More detail lives in [examples/README.md](examples/README.md).

## Context

```rust
use sparsha::prelude::*;

let tree = Provider::new(
    ThemeMode::Dark,
    component()
        .render(|cx| {
            let mode = cx.use_context_or(ThemeMode::Light);
            Text::builder().content(format!("Mode: {mode:?}")).build()
        })
        .call(),
);
```

For shared mutable behavior, provide a `Signal` or another handle type as the context value rather than a large mutable struct.

## Verification

Canonical verification entrypoints:

```bash
./scripts/verify-foundation.sh
./scripts/web-build-all.sh
./scripts/web-smoke.sh
./scripts/wasm-browser-tests.sh
./scripts/web-perf-smoke.sh
./scripts/release-readiness.sh
```

`verify-foundation.sh` runs:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo check -p counter -p layout-probe -p kitchen-sink -p fractal-clock -p hybrid-overlay -p showcase -p todo --target wasm32-unknown-unknown`

`web-smoke.sh` builds and serves `examples/showcase`, `examples/todo`, `examples/kitchen-sink`, `examples/hybrid-overlay`, `examples/counter`, and `examples/layout-probe`, then runs the matching Playwright smoke suite against those pages.

`wasm-browser-tests.sh` runs the checked-in `wasm_bindgen_test` browser entrypoints for `sparsha`.

`web-perf-smoke.sh` builds the checked-in `todo` web example, serves it locally, and stores Lighthouse reports under `artifacts/lighthouse/`.

`release-readiness.sh` composes the foundation verification, browser smoke suite, wasm browser tests, and perf/startup smoke checks into the local pre-release entrypoint that mirrors the checked-in GitHub Actions release-readiness workflow.

GitHub Actions is the canonical hosted gate for 1.0:

- `.github/workflows/ci.yml` runs formatting, clippy, workspace verification, wasm example checks, browser smoke, wasm browser tests, and macOS native verification
- `.github/workflows/release-readiness.yml` runs the sign-off superset, wasm browser tests, and uploads Playwright/perf artifacts
- `.github/workflows/showcase-pages.yml` builds `examples/showcase` and publishes its static `dist/` output to GitHub Pages on pushes to `main` and on manual dispatch

For repository Pages hosting, the showcase stays static-host friendly by using hash routes such as `/#/components` and `/#/rendering`.

## More

- [docs/api-surface.md](docs/api-surface.md)
- [docs/release-checklist.md](docs/release-checklist.md)
- [examples/README.md](examples/README.md)
- [crates/sparsha/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/lib.rs)

## License

MIT
