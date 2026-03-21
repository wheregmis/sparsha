# Sparsh 1.0 Roadmap

## Master Checklist
- [x] Milestone 1: Freeze The Foundation
- [x] Milestone 2: Finish Input, Focus, And Editing
- [ ] Milestone 3: Make Accessibility Real
- [ ] Milestone 4: Finish Web Parity
- [ ] Milestone 5: Polish Core Widgets
- [ ] Milestone 6: Release Engineering And Quality Gates

## Current Baseline
- Sparsh already has a real end-to-end stack: app runner, router, signals, layout, rendering, text shaping, input, widgets, and core GPU primitives.
- The main package surface is consolidated through `sparsh::prelude`, so application code can stay small and readable.
- Native and web runtimes both exist, with web supporting retained DOM rendering and a hybrid GPU surface path.
- The repo ships four runnable example apps that cover the main product shapes: general UI, draw-heavy scenes, hybrid overlays, and a small data-driven app.
- The widget set is already usable for real apps: `Container`, `Button`, `Checkbox`, `Text`, `TextInput`, `TextArea`, `List`, `Scroll`, and `DrawSurface`.
- Module-level tests exist across the core crates, which gives Sparsh a good starting quality baseline rather than a blank slate.

## What Looks Production-Ready Today
- Core layout, render, text, and signal plumbing are implemented and exercised in code and tests.
- The public widget API is coherent and theme-aware, with basic state handling for buttons, checkboxes, text, lists, and containers.
- Router history, back/forward navigation, and fallback handling are in place.
- The text stack has shaping, measurement, glyph caching, and GPU rendering.
- The task runtime is functional on native and web, and the hybrid web surface path is operational.

## What Still Needs Work Before 1.0
- [x] Focus traversal exists in the input model, and it is now wired end to end through the app loops.
- [ ] Accessibility has definitions and data structures, but it is not yet integrated into the runtime and widget tree.
- [x] `TextInput` now covers the expected editing shortcuts and selection behavior, and multiline editing ships as `TextArea`.
- [ ] `Scroll` is usable, but `ScrollDirection::Both` is not fully symmetric and the interaction model is still basic.
- [ ] Router paths are static-only; dynamic route patterns are rejected.
- [ ] The web runtime still behaves more like a visual retained layer than a fully semantic browser-native surface.
- [ ] There is no checked-in CI workflow or release automation, so quality gates are still manual.
- [x] The workspace itself is healthy right now: `cargo check --workspace` and `cargo test --workspace` both pass.

## 1.0 Definition
- [ ] Applications can build and run on native and web with the same widget tree and predictable behavior.
- [ ] Keyboard navigation, pointer interaction, and text editing work across the core widgets without special cases.
- [ ] Accessibility is implemented for the built-in widgets and is verified on both native and web.
- [ ] The web path supports the same interaction semantics as native, including text input and composition flows.
- [ ] The public API is stable enough to document as 1.0 and support semver expectations.
- [ ] CI, smoke tests, and release checks exist and run automatically.

## Milestone 1: Freeze The Foundation
### Tasks
- [x] Freeze the public API surface in `sparsh`, `sparsh-widgets`, `sparsh-input`, `sparsh-layout`, `sparsh-render`, `sparsh-text`, `sparsh-core`, and `sparsh-signals`.
- [x] Document the supported platforms, widget set, and web story in the main README and example docs.
- [x] Audit remaining panic paths, `expect` calls, and `unreachable!` cases that would be unacceptable in a 1.0 release.
- [x] Decide which APIs are truly public and which ones should stay internal until after 1.0.

### Exit Criteria
- [x] The main examples compile cleanly on native and web.
- [x] The documented surface matches the shipped implementation.

## Milestone 2: Finish Input, Focus, And Editing
### Tasks
- [x] Wire `FocusManager` into the app loops so focusable widgets are registered and tab order actually works.
- [x] Route `Tab` and `Shift+Tab` through the semantic action layer instead of leaving them as unused model support.
- [x] Complete pointer capture behavior end to end so drag-like interactions behave consistently.
- [x] Expand `TextInput` to cover copy, cut, paste, undo, redo, selection extension, and a clearer cursor model.
- [x] Add composition and IME handling so text input is correct on web and native.
- [x] Decide whether 1.0 includes multiline text input now or whether it is explicitly deferred.

### Exit Criteria
- [x] Buttons, checkboxes, text input, and focus traversal all work in a way that matches user expectations on both platforms.

## Milestone 3: Make Accessibility Real
### Tasks
- [x] Connect the `Accessible` trait and `AccessibilityManager` to the widget tree and runtime.
- [x] Give built-in widgets stable roles, labels, values, and actions.
- [x] Ensure focus state, disabled state, and value state are exposed consistently.
- [x] Map the accessibility model to native platform accessibility and to semantic web output.
- [ ] Verify the result with screen readers and focused accessibility smoke tests.

### Exit Criteria
- [ ] The built-in widgets are understandable and operable through assistive technologies without custom app code.

## Milestone 4: Finish Web Parity
### Tasks
- [ ] Replace the current browser keyboard/text path with a composition-aware input pipeline.
- [ ] Keep the retained DOM renderer aligned with actual runtime state, not just visual output.
- [ ] Harden hybrid surface startup, resize, recovery, and worker failure handling.
- [ ] Add web integration tests for retained DOM, hybrid overlays, routing, and text input behavior.
- [ ] Make the web release story reproducible from the checked-in repo, not from ad hoc local scripts.

### Exit Criteria
- [ ] Web examples behave like native examples in the core interaction flows that matter for 1.0.

## Milestone 5: Polish Core Widgets
### Tasks
- [ ] Finish `Scroll` so horizontal, vertical, and combined-axis behavior is consistent and predictable.
- [ ] Decide whether `List` should stay a simple owned-children container or grow virtualization for larger data sets.
- [ ] Tighten widget state handling for hover, focus, disabled, and pressed states so all core controls feel consistent.
- [ ] Revisit theme defaults and size metrics so the shipped widgets read as one design system rather than separate demos.

### Exit Criteria
- [ ] The built-in widgets feel intentional enough to be the default surface of a 1.0 app.

## Milestone 6: Release Engineering And Quality Gates
### Tasks
- [ ] Add CI workflows for formatting, linting, unit tests, example smoke tests, and web build checks.
- [ ] Add a release checklist and changelog/release notes so 1.0 is a documented event, not just a git tag.
- [ ] Add a small set of performance and startup smoke tests for render, layout, text, and web boot.
- [ ] Decide how task runtime behavior is supported and documented so it is not just a demo-only convenience.
- [ ] Remove the `winit` beta dependency or explicitly justify why it stays pinned.

### Exit Criteria
- [ ] A clean CI run and the release checklist are both enough to sign off on 1.0.

## Recommended Release Order
- [ ] First stabilize the public API and input/focus behavior.
- [ ] Then make accessibility real, because it depends on stable widget semantics and focus.
- [ ] Then finish web parity, because it depends on the same event and accessibility model.
- [ ] Then polish widgets and release engineering together so the product is easy to trust.
- [ ] Finally cut a release candidate, run the full smoke suite, and tag 1.0 only after the candidate has no open release-blocking issues.

## Repo Evidence
- Core runtime: [crates/sparsh/src/app.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/app.rs), [crates/sparsh/src/web_app.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/web_app.rs), [crates/sparsh/src/tasks.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/tasks.rs)
- Widgets: [crates/sparsh-widgets/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-widgets/src/lib.rs), [crates/sparsh-widgets/src/text_input.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-widgets/src/text_input.rs), [crates/sparsh-widgets/src/scroll.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-widgets/src/scroll.rs), [crates/sparsh-widgets/src/draw_surface.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-widgets/src/draw_surface.rs)
- Input and focus: [crates/sparsh-input/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-input/src/lib.rs), [crates/sparsh-input/src/action.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-input/src/action.rs), [crates/sparsh-input/src/focus.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-input/src/focus.rs)
- Rendering and text: [crates/sparsh-render/src/renderer.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-render/src/renderer.rs), [crates/sparsh-render/src/shape_pass.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-render/src/shape_pass.rs), [crates/sparsh-render/src/text_pass.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-render/src/text_pass.rs), [crates/sparsh-text/src/system.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-text/src/system.rs)
- Layout and core: [crates/sparsh-layout/src/tree.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-layout/src/tree.rs), [crates/sparsh-core/src/types.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-core/src/types.rs), [crates/sparsh-core/src/wgpu_init.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh-core/src/wgpu_init.rs)
- Accessibility and web: [crates/sparsh/src/accessibility.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/accessibility.rs), [crates/sparsh/src/dom_renderer.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/dom_renderer.rs), [crates/sparsh/src/web_surface_manager.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsh/src/web_surface_manager.rs)
- Docs and examples: [README.md](/Users/wheregmis/Documents/GitHub/spark/README.md), [examples/README.md](/Users/wheregmis/Documents/GitHub/spark/examples/README.md)
