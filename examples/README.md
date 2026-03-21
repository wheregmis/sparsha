# Spark Examples

This directory contains runnable Spark example applications.

## Kitchen Sink

**Path:** `examples/kitchen-sink`

A single example application demonstrating Spark's major features: widgets, layout, styling, navigation, and event handling.

**Run:**
```bash
cargo run -p kitchen-sink --release
```

For development (faster compile, slower runtime):
```bash
cargo run -p kitchen-sink
```

**Features:**
- Tab-based navigation
- Widget showcase, layout patterns, form handling, data visualization
- Production-oriented patterns and accessibility

## Building

```bash
cargo build -p kitchen-sink --release
```

To check the entire workspace:
```bash
cargo check --workspace
```

## Platform notes

- **macOS:** Metal backend
- **Windows:** DirectX 12 (fallback to DX11)
- **Linux:** Vulkan

Ensure graphics drivers and system libraries (e.g. Vulkan on Linux) are installed as needed.


## Web (WASM) with Trunk

Kitchen Sink supports WebAssembly via `wasm-bindgen` and can be served with Trunk.

From `examples/kitchen-sink`:
```bash
rustup target add wasm32-unknown-unknown
trunk serve
```

Then open `http://127.0.0.1:8080`.

## Todo

**Path:** `examples/todo`

A cross-platform todo app demonstrating dynamic list rendering with reusable `Checkbox` and `List`
widgets, filter controls, and signal-driven state updates (no `Arc/Mutex` action queue).

**Run:**
```bash
cargo run -p todo
```

For web (WASM + Trunk):
```bash
cd examples/todo
rustup target add wasm32-unknown-unknown
trunk serve
```

Then open `http://127.0.0.1:8081`.
