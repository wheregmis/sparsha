# 1.0 Candidate API Surface

This document records the curated crate-root surface that Sparsha treats as the 1.0 contract. Raw implementation modules and unfinished subsystems are not part of the semver promise yet.

## `sparsha`

Stable for 1.0:

- `App`
- `AppRunError`
- `Router`, `Route`, `Navigator`, `hash_to_path`, `path_to_hash`
- component authoring helpers: `component`, `Component`, `ComponentContext`, `TaskHook`
- theme and accessibility configuration types re-exported from `sparsha-widgets`
- task runtime types: `TaskRuntime`, `TaskRuntimeInitError`, `TaskHandle`, `TaskResult`, `TaskStatus`, `TaskKey`, `TaskId`, `TaskPayload`, `TaskPolicy`, `Generation`
  - supported built-in task kinds in 1.0: `echo`, `sleep_echo`, `analyze_text`
  - custom task registration is not part of the 1.0 contract
- `prelude`
- sub-crate re-exports: `core`, `input`, `layout`, `render`, `signals`, `text`, `widgets`
- `init_web` on `wasm32`

Internal/provisional:

- wasm DOM renderer internals
- native AccessKit adapter internals
- web semantic DOM internals
- hybrid surface manager internals
- internal platform adapters under `crates/sparsha/src/platform/`
- internal runtime orchestration under `crates/sparsha/src/runtime_core.rs`, including `RuntimeHost`

## `sparsha-core`

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

## `sparsha-input`

Stable for 1.0:

- action types and helpers
- `InputEvent`
- keyboard, pointer, and modifier types re-exported from `ui_events`
- `FocusManager`
- hit-testing helpers
- `ui_events`

Internal/provisional:

- `ui_events_winit`

## `sparsha-layout`

Stable for 1.0:

- `LayoutTree`
- `ComputedLayout`
- `WidgetId`
- `styles`
- `taffy`

## `sparsha-render`

Stable for 1.0:

- `DrawCommand`
- `DrawList`
- `TextRun`
- `Renderer`
- `ShapePass`
- `TextPass`

## `sparsha-text`

Stable for 1.0:

- `TextSystem`
- `TextStyle`
- `ShapedText`
- `GlyphAtlas`
- `parley`

## `sparsha-signals`

Stable for 1.0:

- the public signal/runtime API exposed at the crate root, including `Signal`, `ReadSignal`, `WriteSignal`, `Memo`, `Effect`, `RuntimeHandle`, `DirtyFlags`, and `SubscriberKind`

## `sparsha-widgets`

Stable for 1.0:

- widgets/helpers: `Container`, `Button`, `Checkbox`, `Text`, `TextInput`, `List`, `Scroll`, `DrawSurface`, `ForEach`
- editing/accessibility widgets: `TextArea`, `Semantics`
- accessibility metadata types: `AccessibilityInfo`, `AccessibilityRole`, `AccessibilityAction`
- `IntoWidget`
- widget/theme/context types re-exported from the crate root
- `styles`, `taffy`, and `WidgetId` convenience re-exports

Internal/provisional:

- future widget-state behavior beyond the current shipped implementation
