# Sparsh Examples

> Runnable reference apps for the frozen Milestone 1 surface.

Each example is a normal Cargo binary. The examples are intended to demonstrate the stable crate-root APIs rather than internal modules.

## Included Examples

| Example | What it shows |
|---|---|
| `kitchen-sink` | Core widget set, layout composition, theming, scrolling, and input |
| `fractal-clock` | Draw-heavy rendering with `DrawSurface` and reactive state |
| `hybrid-overlay` | DOM-backed UI with a hybrid GPU surface on the web path |
| `todo` | Signals, routing, list rendering, and background task usage in a small app |

## Native

```bash
cargo run -p kitchen-sink
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay
cargo run -p todo
```

## Web

Each example directory contains the checked-in files needed for a Trunk flow:

- `index.html`
- `Trunk.toml`
- `sparsh-worker.js`

Example:

```bash
cd examples/kitchen-sink
rustup target add wasm32-unknown-unknown
trunk serve
```

The other examples follow the same pattern from their own directories.

## What The Examples Intentionally Do Not Promise Yet

- Accessibility smoke verification is still manual in this milestone
- `todo` intentionally keeps `TextInput` single-line while `kitchen-sink` demonstrates `TextArea`
- Router usage stays on static paths
- CI/lighthouse automation is not required to consider the examples healthy for Milestone 1

## Verification

From the repo root, run:

```bash
./scripts/verify-foundation.sh
```

That covers the native workspace checks plus wasm compile checks for all four examples.
