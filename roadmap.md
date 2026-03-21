# Sparsha 1.0 Roadmap

## Master Checklist
- [x] Milestone 1: Freeze The Foundation
- [x] Milestone 2: Finish Input, Focus, And Editing
- [x] Milestone 3: Make Accessibility Real
- [x] Milestone 4: Finish Web Parity
- [x] Milestone 5: Polish Core Widgets
- [ ] Milestone 6: Release Engineering And Quality Gates

Milestone checkboxes track implementation landing in the repo. The unchecked items below are the remaining verification, release, and sign-off work.

## Current Baseline
- Sparsha already has a real end-to-end stack: app runner, router, signals, layout, rendering, text shaping, input, widgets, and core GPU primitives.
- The main package surface is consolidated through `sparsha::prelude`, so application code can stay small and readable.
- Native and web runtimes both exist, with web supporting retained DOM rendering and a hybrid GPU surface path.
- The repo ships four runnable example apps that cover the main product shapes: general UI, draw-heavy scenes, hybrid overlays, and a small data-driven app.
- The widget set is already usable for real apps: `Container`, `Button`, `Checkbox`, `Text`, `TextInput`, `TextArea`, `List`, `Scroll`, and `DrawSurface`.
- Module-level tests exist across the core crates, which gives Sparsha a good starting quality baseline rather than a blank slate.

## What Looks Production-Ready Today
- Core layout, render, text, and signal plumbing are implemented and exercised in code and tests.
- The public widget API is coherent and theme-aware, with basic state handling for buttons, checkboxes, text, lists, and containers.
- Router history, back/forward navigation, and fallback handling are in place.
- The text stack has shaping, measurement, glyph caching, and GPU rendering.
- The task runtime is functional on native and web, and the hybrid web surface path is operational.

## What Still Needs Work Before 1.0
- [x] Focus traversal exists in the input model, and it is now wired end to end through the app loops.
- [ ] Accessibility is now integrated into the runtime and built-in widgets, but screen-reader verification and assistive-technology sign-off are still pending.
- [x] `TextInput` now covers the expected editing shortcuts and selection behavior, and multiline editing ships as `TextArea`.
- [x] `Scroll` now supports polished vertical, horizontal, and both-axis behavior with interactive scrollbars.
- [x] `List` now keeps the simple owned-children mode for small data and also supports fixed-extent virtualization for larger data sets.
- [ ] Router paths are static-only; dynamic route patterns are rejected.
- [ ] The web runtime now has retained DOM parity work and a semantic DOM layer, but final browser parity and manual sign-off are still open.
- [ ] GitHub Actions and release-readiness workflows are now checked in, but the first clean sign-off run plus the remaining manual accessibility/web parity checks are still pending.
- [x] The workspace itself is healthy right now: `cargo check --workspace` and `cargo test --workspace` both pass.

## 1.0 Definition
- [ ] Applications can build and run on native and web with the same widget tree and predictable behavior.
- [x] Keyboard navigation, pointer interaction, and text editing work across the core widgets without special cases.
- [ ] Accessibility is implemented for the built-in widgets and is verified on both native and web.
- [ ] The web path supports the same interaction semantics as native, including text input and composition flows.
- [ ] The public API is stable enough to document as 1.0 and support semver expectations.
- [x] CI, smoke tests, and release checks exist and run automatically.

## Milestone 1: Freeze The Foundation
### Tasks
- [x] Freeze the public API surface in `sparsha`, `sparsha-widgets`, `sparsha-input`, `sparsha-layout`, `sparsha-render`, `sparsha-text`, `sparsha-core`, and `sparsha-signals`.
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
- [ ] The built-in widgets are understandable and operable through assistive technologies without custom app code, and this has been confirmed with the planned accessibility smoke tests.

## Milestone 4: Finish Web Parity
### Tasks
- [x] Replace the current browser keyboard/text path with a composition-aware input pipeline.
- [x] Keep the retained DOM renderer aligned with actual runtime state, not just visual output.
- [x] Harden hybrid surface startup, resize, recovery, and worker failure handling.
- [x] Add web integration tests for retained DOM, hybrid overlays, routing, and text input behavior.
- [x] Make the web release story reproducible from the checked-in repo, not from ad hoc local scripts.

### Exit Criteria
- [ ] Web examples behave like native examples in the core interaction flows that matter for 1.0, including the remaining manual parity smoke checks.

## Milestone 5: Polish Core Widgets
### Tasks
- [x] Finish `Scroll` so horizontal, vertical, and combined-axis behavior is consistent and predictable.
- [x] Decide whether `List` should stay a simple owned-children container or grow virtualization for larger data sets.
- [x] Tighten widget state handling for hover, focus, disabled, and pressed states so all core controls feel consistent.
- [x] Revisit theme defaults and size metrics so the shipped widgets read as one design system rather than separate demos.

### Exit Criteria
- [x] The built-in widgets feel intentional enough to be the default surface of a 1.0 app.

## Milestone 6: Release Engineering And Quality Gates
### Tasks
- [x] Add CI workflows for formatting, linting, unit tests, example smoke tests, and web build checks.
- [x] Add a release checklist and changelog/release notes so 1.0 is a documented event, not just a git tag.
- [x] Add a small set of performance and startup smoke tests for render, layout, text, and web boot.
- [x] Decide how task runtime behavior is supported and documented so it is not just a demo-only convenience.
- [x] Document the current platform/runtime dependency pin rationale, including the stable `winit` pin and its companion adapter versions.

### Exit Criteria
- [ ] A clean CI run and the release checklist are both enough to sign off on 1.0.

## Recommended Release Order
- [ ] First stabilize the public API and input/focus behavior.
- [ ] Then make accessibility real, because it depends on stable widget semantics and focus.
- [ ] Then finish web parity, because it depends on the same event and accessibility model.
- [ ] Then polish widgets and release engineering together so the product is easy to trust.
- [ ] Finally cut a release candidate, run the full smoke suite, and tag 1.0 only after the candidate has no open release-blocking issues.

## Repo Evidence
- Core runtime: [crates/sparsha/src/app.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/app.rs), [crates/sparsha/src/web_app.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/web_app.rs), [crates/sparsha/src/tasks.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/tasks.rs)
- Widgets: [crates/sparsha-widgets/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-widgets/src/lib.rs), [crates/sparsha-widgets/src/text_input.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-widgets/src/text_input.rs), [crates/sparsha-widgets/src/scroll.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-widgets/src/scroll.rs), [crates/sparsha-widgets/src/draw_surface.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-widgets/src/draw_surface.rs)
- Input and focus: [crates/sparsha-input/src/lib.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-input/src/lib.rs), [crates/sparsha-input/src/action.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-input/src/action.rs), [crates/sparsha-input/src/focus.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-input/src/focus.rs)
- Rendering and text: [crates/sparsha-render/src/renderer.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-render/src/renderer.rs), [crates/sparsha-render/src/shape_pass.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-render/src/shape_pass.rs), [crates/sparsha-render/src/text_pass.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-render/src/text_pass.rs), [crates/sparsha-text/src/system.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-text/src/system.rs)
- Layout and core: [crates/sparsha-layout/src/tree.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-layout/src/tree.rs), [crates/sparsha-core/src/types.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-core/src/types.rs), [crates/sparsha-core/src/wgpu_init.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha-core/src/wgpu_init.rs)
- Accessibility and web: [crates/sparsha/src/accessibility.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/accessibility.rs), [crates/sparsha/src/dom_renderer.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/dom_renderer.rs), [crates/sparsha/src/web_surface_manager.rs](/Users/wheregmis/Documents/GitHub/spark/crates/sparsha/src/web_surface_manager.rs)
- Docs and examples: [README.md](/Users/wheregmis/Documents/GitHub/spark/README.md), [examples/README.md](/Users/wheregmis/Documents/GitHub/spark/examples/README.md)
