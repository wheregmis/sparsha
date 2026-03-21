# Sparsh Examples

> Small apps that show how Sparsh behaves in practice.

Each example is a normal Cargo binary. Run them natively, or serve the web builds with Trunk.

## At A Glance

| Example | What it shows |
|---|---|
| `kitchen-sink` | Full widget showcase, layout patterns, navigation, form handling |
| `fractal-clock` | Full-screen GPU-heavy scene with time-based animation and controls |
| `hybrid-overlay` | `DrawSurface` plus retained overlays in a hybrid web layout |
| `todo` | Reactive state, list rendering, filters, and reusable widgets |

## Run Them

Native:

```bash
cargo run -p kitchen-sink
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay
cargo run -p todo
```

Web:

```bash
cd examples/kitchen-sink
rustup target add wasm32-unknown-unknown
trunk serve
```

The other examples follow the same pattern from their own directories.

## Kitchen Sink

Path: `examples/kitchen-sink`

The most complete example in the repo. It demonstrates widget composition, tab navigation, layout structure, and production-style UI patterns.

- Use it when you want to see the public widget API in one place.
- Good starting point for learning `Container`, `Button`, `Text`, `TextInput`, `List`, and `Scroll`.

## Fractal Clock

Path: `examples/fractal-clock`

A full-screen generative demo that turns UTC time into an animated fractal field with layered glow and interactive controls.

- Use it to see Sparsh handle draw-heavy scenes.
- Good reference for animation, composition, and responsive overlays.

## Hybrid Overlay

Path: `examples/hybrid-overlay`

A compact hybrid rendering sample that paints a GPU scene inside `DrawSurface` while keeping panels and text in the retained widget tree.

- Use it to see the web hybrid path in isolation.
- Good reference for embedding custom GPU content into Sparsh UI.

## Todo

Path: `examples/todo`

A cross-platform todo app built around signal-driven state, reusable list items, and simple filtering interactions.

- Use it to see how state, events, and widgets fit together in a realistic app.
- Good reference for a small but complete app structure.

## Notes

- Desktop targets use the native `winit` and `wgpu` stack.
- Web targets use the same app model with retained DOM rendering by default.
- Example code and docs live beside the framework, so the examples stay aligned with the public API.
