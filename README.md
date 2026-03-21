# Sparsh

A GPU-first cross-platform UI framework in Rust, built on `wgpu` and `winit`.

## Features

- **GPU-Accelerated Rendering** - All rendering uses wgpu with instanced drawing for shapes and text
- **Flexbox Layout** - Powered by [taffy](https://github.com/DioxusLabs/taffy) for familiar CSS-like layouts
- **Cross-Platform** - Desktop (Windows, macOS, Linux) and Web (via DOM rendering)
- **Modern Text Rendering** - Using [parley](https://github.com/linebender/parley) + [swash](https://github.com/dfrg/swash) for shaping and rasterization
- **W3C-Compliant Events** - Using [ui-events](https://github.com/endoli/ui-events) for input handling
- **Accessibility** - Using [accesskit](https://github.com/AccessKit/accesskit) for native assistive tech
- **Reactive Signals** - Signal-driven state with automatic frame invalidation

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│              (User code: components, state)                  │
├─────────────────────────────────────────────────────────────┤
│                    sparsh (facade)                           │
│              App runner, context, hooks                      │
├─────────────────────────────────────────────────────────────┤
│ sparsh-signals │ sparsh-widgets │ sparsh-input │ sparsh-layout  │
│ (Signal/Memo) │ (Button, Input)│ (Events, Focus)│ (Flexbox) │
├─────────────────────────────────────────────────────────────┤
│                    sparsh-render                             │
│     DrawList, ShapePass, TextPass, batching, sorting         │
├─────────────────────────────────────────────────────────────┤
│                    sparsh-core                               │
│     Pipeline<U>, wgpu init, uniform buffers, shaders         │
├─────────────────────────────────────────────────────────────┤
│                    Platform                                   │
│      Desktop (wgpu + winit) | Web (DOM + requestAnimationFrame) │
└─────────────────────────────────────────────────────────────┘
```

## Platform Support

- **Desktop**: Windows, macOS, Linux (Vulkan/Metal/DX12)
- **Web**: WebAssembly + DOM rendering

## Crates

| Crate | Description |
|-------|-------------|
| `sparsh` | Main facade crate with App runner |
| `sparsh-core` | GPU primitives, pipelines, vertex buffers |
| `sparsh-render` | DrawList, shape/text rendering passes |
| `sparsh-layout` | Flexbox layout via taffy |
| `sparsh-text` | Font loading, text shaping, glyph atlas |
| `sparsh-input` | Event types, focus management, hit testing |
| `sparsh-signals` | Reactive signal runtime (`Signal`, `Memo`, `Effect`) |
| `sparsh-widgets` | Widget trait and basic widgets |

## Quick Start

```rust
use sparsh::prelude::*;

fn main() {
    App::new()
        .with_title("My App")
        .with_size(800, 600)
        .run(|| {
            Box::new(
                Container::new()
                    .fill()
                    .center()
                    .gap(16.0)
                    .child(Button::new("Click me!"))
                    .child(TextInput::new().placeholder("Enter text..."))
            )
        });
}
```

## Widgets

- **Container** - Flexbox container for layout
- **Button** - Clickable button with hover/press states
- **Text** - Rich text rendering with alignment and styling
- **TextInput** - Single-line text input with cursor
- **Scroll** - Scrollable container

## Try It

```bash
cargo run -p kitchen-sink --release
```

For web (WASM + Trunk):
```bash
cd examples/kitchen-sink
rustup target add wasm32-unknown-unknown
trunk serve
```

## Frame Loop

```
1. EVENT PHASE
   - Collect winit events → InputEvent
   - Dispatch to focused widget / hit-test target
   - Update widget state if needed

2. LAYOUT PHASE
   - Traverse widget tree, collect taffy::Style
   - Call taffy.compute_layout()
   - Produce ComputedLayout tree

3. PAINT PHASE
   - Traverse widget tree with ComputedLayout
   - Each widget emits DrawCommands to DrawList

4. RENDER PHASE
   - Sort DrawList by pipeline (shapes, then text)
   - Batch vertices into GPU buffers
   - Encode render passes
   - Submit to queue, present frame
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `wgpu` | GPU abstraction |
| `winit` | Windowing (desktop + web) |
| `taffy` | Flexbox layout engine |
| `parley` | Text layout + shaping |
| `swash` | Font rasterization |
| `ui-events` | W3C-compliant input events |
| `ui-events-winit` | Winit integration for input events |
| `accesskit` | Accessibility tree + actions |
| `accesskit_winit` | Winit accessibility adapter |
| `bytemuck` | Safe GPU buffer casts |
| `glam` | Math types (Vec2, Mat4) |
| `slotmap` | Handle-based collections |
| `rustc-hash` | Fast hash maps/sets |

## References

- [Makepad](https://github.com/makepad/makepad) - GPU-first UI in Rust
- [gpui](https://github.com/zed-industries/zed) - Zed's UI framework
- [wgpu](https://github.com/gfx-rs/wgpu) - Cross-platform GPU abstraction

## License

MIT
