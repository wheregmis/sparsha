# Sparsha Examples

> Runnable reference apps for the current polished 1.0 candidate surface.

Each example is a normal Cargo binary. The examples are intended to demonstrate the stable crate-root APIs rather than internal modules.

The canonical authoring surface uses a semantic split: `App::builder()`, `Router::builder()`, and `component().render(...).call()` stay bon-first; subtree-scoped typed values use `Provider::new(...)` plus `cx.use_context::<T>()`, `cx.use_context_or(...)`, or `cx.use_context_or_else(...)`, while built-in framework resources stay on dedicated accessors like `cx.viewport()` and `cx.navigator()`; structural tree widgets use semantic constructors such as `Container::column()`, `Container::row()`, `Container::main_axis_alignment(...)`, `Container::cross_axis_alignment(...)`, `Scroll::vertical(...)`, `Scroll::horizontal(...)`, `List::empty()`, `Semantics::new(...)`, and semantic wrappers like `Center::new(...)`, `Padding::all(...)`, `Expanded::new(...)`, `Stack::new()`, and `Positioned::new(...)`; config-heavy widgets use bon builders such as `Text::builder()`, `Button::builder()`, and `List::virtualized_builder()` for the fixed-row virtualized path. Typography variants and paragraph policy stay on that same path through `Text::builder().variant(TextVariant::Header)` and options like `line_height(...)`, `wrap(TextWrap::Word)`, `max_lines(...)`, and overflow policies such as `TextOverflow::Clip` and `TextOverflow::Ellipsis`.

## Included Examples

| Example | What it shows |
|---|---|
| `counter` | Flutter-style starter counter with app bar, centered count, and a bottom-right `+` action |
| `layout-probe` | Center-crosshair layout probe for diagnosing viewport, alignment, and paint-scaling bugs |
| `kitchen-sink` | Polished core widgets, theming, two-axis scroll, text editing, and virtualized lists |
| `fractal-clock` | Draw-heavy rendering with `DrawSurface` and reactive state |
| `hybrid-overlay` | DOM-backed UI with a hybrid GPU surface on the web path |
| `showcase` | Hash-routed public preview surface with component samples, nested `Provider` context demos, and manual rendering checks |
| `todo` | Bon-backed function components, `Provider` context defaults, signals, keyed `ForEach`, routing, and background task hooks in a small app |

## Native

```bash
cargo run -p counter
cargo run -p layout-probe
cargo run -p kitchen-sink
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay
cargo run -p showcase
cargo run -p todo
```

## Mobile (cargo-mobile2)

Install `cargo-mobile2` once:

```bash
cargo install --git https://github.com/tauri-apps/cargo-mobile2
```

Then run any example on Android/iOS:

```bash
./scripts/mobile-run-example.sh kitchen-sink android run
./scripts/mobile-run-example.sh kitchen-sink ios run
./scripts/mobile-run-example.sh kitchen-sink android run wgpu
```

`ios` commands require macOS.
If an example has not been initialized for mobile yet, the helper script runs `cargo mobile init --non-interactive` for that example first and enforces `template-pack = "winit"` by default (or `wgpu` when passed explicitly as the 4th argument).

## Web

Each example directory still contains the checked-in files needed for a Trunk flow:

- `index.html`
- `Trunk.toml`
- `sparsha-worker.js`

Canonical repo-root workflow:

```bash
rustup target add wasm32-unknown-unknown
./scripts/web-build-example.sh kitchen-sink
./scripts/web-serve-dist.sh kitchen-sink 4173
```

Build all checked-in web examples:

```bash
./scripts/web-build-all.sh
```

Run the headless browser smoke suite:

```bash
npm install
npm run web:install
./scripts/web-smoke.sh
```

Run the checked-in browser wasm tests for the `sparsha` crate:

```bash
./scripts/wasm-browser-tests.sh
```

Run the lightweight web perf/startup smoke:

```bash
./scripts/web-perf-smoke.sh
```

Run the full local release-readiness suite:

```bash
./scripts/release-readiness.sh
```

Direct `trunk serve` from an example directory remains useful for manual iteration.

The public `showcase` example is also published through `.github/workflows/showcase-pages.yml`. Because it uses hash routing, the deployed Pages URL remains static-host safe for routes like `/#/rendering`.

## What The Examples Intentionally Do Not Promise Yet

- Accessibility smoke verification is still manual in this milestone
- `todo` intentionally keeps `TextInput` single-line while `kitchen-sink` demonstrates `TextArea`
- `todo` intentionally uses `ForEach` plus `Scroll` instead of the virtualized `List` path that `kitchen-sink` exercises
- Router usage stays on static paths
- Final accessibility and browser parity sign-off still includes manual checks beyond the automated workflows

## Verification

From the repo root, run:

```bash
./scripts/verify-foundation.sh
./scripts/web-smoke.sh
./scripts/wasm-browser-tests.sh
./scripts/web-perf-smoke.sh
./scripts/release-readiness.sh
```

That covers the native workspace checks, wasm compile checks for all seven example binaries, the multi-example browser smoke suite, the checked-in browser wasm tests, and the lightweight perf/startup smoke path used in release readiness. Hosted automation lives in `.github/workflows/ci.yml`, `.github/workflows/release-readiness.yml`, and `.github/workflows/showcase-pages.yml`.
