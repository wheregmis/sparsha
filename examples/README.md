# Sparsh Examples

> Runnable reference apps for the current polished 1.0 candidate surface.

Each example is a normal Cargo binary. The examples are intended to demonstrate the stable crate-root APIs rather than internal modules.

## Included Examples

| Example | What it shows |
|---|---|
| `kitchen-sink` | Polished core widgets, theming, two-axis scroll, text editing, and virtualized lists |
| `fractal-clock` | Draw-heavy rendering with `DrawSurface` and reactive state |
| `hybrid-overlay` | DOM-backed UI with a hybrid GPU surface on the web path |
| `todo` | Signals, routing, simple owned-child lists, and background task usage in a small app |

## Native

```bash
cargo run -p kitchen-sink
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay
cargo run -p todo
```

## Web

Each example directory still contains the checked-in files needed for a Trunk flow:

- `index.html`
- `Trunk.toml`
- `sparsh-worker.js`

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

Run the lightweight web perf/startup smoke:

```bash
./scripts/web-perf-smoke.sh
```

Run the full local release-readiness suite:

```bash
./scripts/release-readiness.sh
```

Direct `trunk serve` from an example directory remains useful for manual iteration.

## What The Examples Intentionally Do Not Promise Yet

- Accessibility smoke verification is still manual in this milestone
- `todo` intentionally keeps `TextInput` single-line while `kitchen-sink` demonstrates `TextArea`
- `todo` intentionally keeps the simple owned-children `List` path while `kitchen-sink` exercises the virtualized mode
- Router usage stays on static paths
- Final accessibility and browser parity sign-off still includes manual checks beyond the automated workflows

## Verification

From the repo root, run:

```bash
./scripts/verify-foundation.sh
./scripts/web-smoke.sh
./scripts/web-perf-smoke.sh
./scripts/release-readiness.sh
```

That covers the native workspace checks, wasm compile checks for all four examples, the browser smoke suite, and the lightweight perf/startup smoke path used in release readiness. Hosted automation lives in `.github/workflows/ci.yml` and `.github/workflows/release-readiness.yml`.
