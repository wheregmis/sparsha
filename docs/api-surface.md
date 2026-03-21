# Milestone 1 API Surface

This document records the foundation freeze for Milestone 1. The 1.0 contract is the curated crate-root API described here. Raw implementation modules and unfinished subsystems are not part of the semver promise yet.

## `sparsh`

Stable for 1.0:

- `App`
- `AppRunError`
- `Router`, `Route`, `Navigator`, `hash_to_path`, `path_to_hash`
- theme configuration types re-exported from `sparsh-widgets`
- task runtime types: `TaskRuntime`, `TaskRuntimeInitError`, `TaskHandle`, `TaskResult`, `TaskStatus`, `TaskKey`, `TaskId`, `TaskPayload`, `TaskPolicy`, `Generation`
- `prelude`
- sub-crate re-exports: `core`, `input`, `layout`, `render`, `signals`, `text`, `widgets`
- `init_web` on `wasm32`

Internal/provisional:

- `sparsh::accessibility`
- wasm DOM renderer internals
- hybrid surface manager internals

## `sparsh-core`

Stable for 1.0:

- `Color`, `Rect`, `Point`, `GlobalUniforms`
- `DynamicBuffer`, `StaticBuffer`, `QuadBuffers`
- `Pipeline`, `UniformBuffer`
- `Vertex2D`, `ShapeInstance`, `GlyphInstance`
- `init_wgpu`, `init_wgpu_headless`, `SurfaceState`, `WgpuInitError`
- `glam`
- `wgpu`

Internal/provisional:

- raw `buffer`, `pipeline`, `types`, `vertex`, and `wgpu_init` module paths

## `sparsh-input`

Stable for 1.0:

- action types and helpers
- `InputEvent`
- keyboard, pointer, and modifier types re-exported from `ui_events`
- `FocusManager`
- hit-testing helpers
- `ui_events`

Internal/provisional:

- `ui_events_winit`

## `sparsh-layout`

Stable for 1.0:

- `LayoutTree`
- `ComputedLayout`
- `WidgetId`
- `styles`
- `taffy`

## `sparsh-render`

Stable for 1.0:

- `DrawCommand`
- `DrawList`
- `TextRun`
- `Renderer`
- `ShapePass`
- `TextPass`

## `sparsh-text`

Stable for 1.0:

- `TextSystem`
- `TextStyle`
- `ShapedText`
- `GlyphAtlas`
- `parley`

## `sparsh-signals`

Stable for 1.0:

- the public signal/runtime API exposed at the crate root, including `Signal`, `ReadSignal`, `WriteSignal`, `Memo`, `Effect`, `RuntimeHandle`, `DirtyFlags`, and `SubscriberKind`

## `sparsh-widgets`

Stable for 1.0:

- widgets: `Container`, `Button`, `Checkbox`, `Text`, `TextInput`, `List`, `Scroll`, `DrawSurface`
- widget/theme/context types re-exported from the crate root
- `styles`, `taffy`, and `WidgetId` convenience re-exports

Internal/provisional:

- unfinished accessibility semantics
- future widget-state and editing behavior beyond the current shipped implementation
