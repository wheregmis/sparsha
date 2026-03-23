# 1.0 Candidate API Surface

This document records the curated crate-root surface that Sparsha treats as the 1.0 contract. Raw implementation modules and unfinished subsystems are not part of the semver promise yet.

## `sparsha`

Stable for 1.0:

- `App`
- `AppRunError`
- `Router`, `Route`, `Navigator`, `hash_to_path`, `path_to_hash`
- authoring lanes:
  - primary composition lane: bon-backed `App::builder()`, `Router::builder()`, `component().render(...).call()`, plus a semantic structural/widget split:
    - structural tree widgets use semantic constructors plus fluent child/content composition, for example `Container::column()`, `Container::row()`, `Container::main_axis_alignment(...)`, `Container::cross_axis_alignment(...)`, `Scroll::vertical(...)`, `Scroll::horizontal(...)`, `List::empty()`, `Provider::new(...)`, and `Semantics::new(...)`
    - config-heavy and leaf widgets use bon builders such as `Text::builder()`, `Button::builder()`, `Checkbox::builder()`, `TextInput::builder()`, `TextArea::builder()`, and `List::virtualized_builder()`
    - function components can read provider-scoped subtree values through `ComponentContext::use_context::<T>() -> Option<T>`, `use_context_or(...)`, and `use_context_or_else(...)`
    - built-in framework resources remain on dedicated component accessors such as `viewport()`, `navigator()`, and `task_runtime()`
    - responsive text roles stay on the builder surface through `Text::builder().variant(TextVariant::Header)` rather than shortcut constructors
  - specialized lane: dedicated primitives such as `ForEach`, `DrawSurface`, animation helpers, and theme/style value types
  - expert lane: low-level `Widget` and context APIs for manual custom widgets
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

Contributor rule:

- new public authoring APIs must declare which lane they belong to
- normal UI composition must not gain parallel public entrypoints that overlap in behavior
- prefer semantic constructors for structural widgets that primarily accumulate child trees; prefer bon builders for config-heavy widgets where typestate/defaulting materially improves clarity and safety
- when using `Provider`, prefer passing `Signal` or other shared handles for mutable behavior rather than large mutable structs by value

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

- widgets/helpers: `Container`, `Button`, `Checkbox`, `Text`, `TextInput`, `List`, `Scroll`, `Provider`, `DrawSurface`, `ForEach`
- editing/accessibility widgets: `TextArea`, `Semantics`
- accessibility metadata types: `AccessibilityInfo`, `AccessibilityRole`, `AccessibilityAction`
- `IntoWidget`
- widget/theme/context types re-exported from the crate root, including `MainAxisAlignment` and `CrossAxisAlignment`
- `styles`, `taffy`, and `WidgetId` convenience re-exports

Internal/provisional:

- future widget-state behavior beyond the current shipped implementation
