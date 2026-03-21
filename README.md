# Sparsh

> GPU-first, cross-platform UI for Rust.
>
> Native on desktop. Retained DOM on web. A hybrid `DrawSurface` path when you want GPU-heavy scenes without giving up the widget tree.

Sparsh is a UI framework built around a small set of focused crates:
`wgpu` for rendering, `winit` for windowing, `taffy` for flexbox layout,
`parley` + `swash` for text, `ui-events` for input, and `accesskit` for accessibility.

## Why Sparsh

- **One API across desktop and web** - Write against the same widget tree and run it on native platforms or in the browser.
- **GPU-first rendering** - Shape and text passes are designed around batched GPU work.
- **Flexible web story** - Retained DOM rendering is the default, with `DrawSurface` for hybrid GPU scenes.
- **Practical widget set** - `Container`, `Button`, `Checkbox`, `Text`, `TextInput`, `List`, and `Scroll` are ready to compose.
- **Reactive by design** - `Signal`, `Memo`, and `Effect` make app state easy to wire into rendering.
- **Accessible from the start** - Widgets can describe roles, labels, values, and actions for assistive tech.

## What It Looks Like

```rust
use sparsh::prelude::*;

fn main() {
    #[cfg(target_arch = "wasm32")]
    sparsh::init_web();

    App::new()
        .with_title("Hello Sparsh")
        .with_size(960, 640)
        .run(|| {
            Box::new(
                Container::new()
                    .fill()
                    .center()
                    .gap(16.0)
                    .child(Text::new("Build UI with a GPU-first stack."))
                    .child(Button::new("Click me"))
                    .child(TextInput::new().placeholder("Type here...")),
            )
        });
}
```

## Stack

```text
Application code
  ↳ sparsh
      ↳ sparsh-widgets   Widget tree, layout hooks, draw surfaces
      ↳ sparsh-signals   Signal runtime and reactivity
      ↳ sparsh-input     Events, focus, hit testing, shortcuts
      ↳ sparsh-layout    Flexbox layout via taffy
      ↳ sparsh-render    Shape and text draw passes
      ↳ sparsh-text      Font loading, shaping, glyph atlas
      ↳ sparsh-core      GPU primitives and wgpu init
```

## Crates

| Crate | Role |
|---|---|
| `sparsh` | App runner and public facade |
| `sparsh-core` | Low-level GPU primitives and platform setup |
| `sparsh-render` | Draw list, shape pass, text pass, batching |
| `sparsh-layout` | Widget layout tree and flexbox integration |
| `sparsh-text` | Font loading, shaping, glyph atlas management |
| `sparsh-input` | Input events, focus management, hit testing |
| `sparsh-signals` | Reactive signal runtime |
| `sparsh-widgets` | Reusable widgets and painting/build contexts |

## Widgets

The public widget layer currently includes:

- `Container`
- `Button`
- `Checkbox`
- `Text`
- `TextInput`
- `List`
- `Scroll`
- `DrawSurface`

## Runtime Model

Sparsh follows a straightforward frame pipeline:

1. Collect input events.
2. Rebuild widget state when signals invalidate.
3. Compute layout with `taffy`.
4. Paint widgets into draw commands.
5. Render shapes and text through the GPU.

On web, the app runner keeps the DOM responsive and uses a worker-backed task runtime. For draw-heavy scenes, `DrawSurface` lets you embed GPU content inside otherwise retained DOM UI.

## Examples

Each example is a separate Cargo binary:

```bash
cargo run -p kitchen-sink --release
cargo run -p fractal-clock --release
cargo run -p hybrid-overlay --release
cargo run -p todo --release
```

For web examples with Trunk:

```bash
cd examples/kitchen-sink
rustup target add wasm32-unknown-unknown
trunk serve
```

Other example directories follow the same pattern.

## Platform Support

- **Desktop**: Windows, macOS, Linux
- **Web**: WebAssembly with DOM rendering and optional hybrid GPU surfaces

## Dependencies

Sparsh is built on:

| Crate | Purpose |
|---|---|
| `wgpu` | GPU abstraction |
| `winit` | Native and web windowing/events |
| `taffy` | Flexbox layout engine |
| `parley` | Text layout and shaping |
| `swash` | Font rasterization |
| `ui-events` | W3C-compliant input event types |
| `ui-events-winit` | Input bridge for winit |
| `accesskit` | Accessibility tree and actions |
| `accesskit_winit` | Accessibility integration for winit |
| `glam` | Math types |
| `bytemuck` | Safe GPU buffer casts |
| `slotmap` | Handle-based collections |
| `rustc-hash` | Fast hash maps |

## More

- [examples/README.md](/Users/wheregmis/Documents/GitHub/spark/examples/README.md)
- [crates/sparsh/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/lib.rs)
- [crates/sparsh-widgets/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-widgets/src/lib.rs)

## License

MIT
