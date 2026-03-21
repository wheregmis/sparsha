# Scroll Content Transformation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement content transformation in the Scroll widget so scrolled content is properly offset/translated during rendering.

**Architecture:** Add PushTranslation/PopTranslation draw commands to the rendering pipeline, add a paint_after_children() hook to the Widget trait, and update the Scroll widget to use translation for offsetting child content based on scroll position.

**Tech Stack:** Rust, wgpu (GPU rendering), custom draw list architecture, Taffy layout engine

---

## Task 1: Add Translation Draw Commands

**Files:**
- Modify: `crates/sparsh-render/src/commands.rs:1-152`

**Step 1: Add translation variants to DrawCommand enum**

Add after the `PopClip` variant (around line 25):

```rust
/// Push a translation offset (affects all subsequent draw commands).
PushTranslation {
    offset: (f32, f32),
},
/// Pop the current translation offset.
PopTranslation,
```

**Step 2: Add convenience methods to DrawList impl**

Add after the `pop_clip()` method (around line 129):

```rust
/// Push a translation offset for subsequent draw commands.
pub fn push_translation(&mut self, offset: (f32, f32)) {
    self.push(DrawCommand::PushTranslation { offset });
}

/// Pop the current translation offset.
pub fn pop_translation(&mut self) {
    self.push(DrawCommand::PopTranslation);
}
```

**Step 3: Verify compilation**

Run: `cargo check -p sparsh-render`
Expected: SUCCESS with no errors

**Step 4: Commit**

```bash
git add crates/sparsh-render/src/commands.rs
git commit -m "feat(render): add PushTranslation and PopTranslation draw commands"
```

---

## Task 2: Add Translation Support to PaintContext

**Files:**
- Modify: `crates/sparsh-widgets/src/context.rs:27-179`

**Step 1: Add translation methods to PaintContext impl**

Add after the `pop_clip()` method (around line 99):

```rust
/// Push a translation offset for child content.
/// The offset is in physical pixels and affects all subsequent draw commands.
pub fn push_translation(&mut self, offset: (f32, f32)) {
    self.draw_list.push_translation(offset);
}

/// Pop the current translation offset.
pub fn pop_translation(&mut self) {
    self.draw_list.pop_translation();
}
```

**Step 2: Verify compilation**

Run: `cargo check -p sparsh-widgets`
Expected: SUCCESS with no errors

**Step 3: Commit**

```bash
git add crates/sparsh-widgets/src/context.rs
git commit -m "feat(widgets): add translation methods to PaintContext"
```

---

## Task 3: Add paint_after_children Hook to Widget Trait

**Files:**
- Modify: `crates/sparsh-widgets/src/widget.rs:1-154`

**Step 1: Add paint_after_children method to Widget trait**

Add after the `paint()` method (around line 100):

```rust
/// Called after children have been painted.
/// Use this to clean up transformations or clips pushed in paint().
/// Default implementation does nothing.
fn paint_after_children(&self, _ctx: &mut PaintContext) {
    // Default: no-op
}
```

**Step 2: Verify compilation**

Run: `cargo check -p sparsh-widgets`
Expected: SUCCESS with no errors (default impl means no changes needed to existing widgets)

**Step 3: Commit**

```bash
git add crates/sparsh-widgets/src/widget.rs
git commit -m "feat(widgets): add paint_after_children hook to Widget trait"
```

---

## Task 4: Update Framework to Call paint_after_children

**Files:**
- Modify: `crates/sparsh/src/app.rs:278-370`

**Step 1: Call paint_after_children after painting children**

Modify the `paint_widget` function. After the children painting loop (around line 349-365), add:

Find this section:
```rust
// Paint children
for child in widget.children() {
    paint_widget(
        child.as_ref(),
        layout_tree,
        focus,
        ctx.draw_list,
        scale_factor,
        text_system_ptr,
        device_ptr,
        queue_ptr,
        elapsed_time,
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        native_view_manager,
    );
}
```

After the for loop, add:

```rust
// Call after-paint hook for cleanup (e.g., pop transforms/clips)
widget.paint_after_children(&mut ctx);
```

**Step 2: Verify compilation**

Run: `cargo check -p sparsh`
Expected: SUCCESS with no errors

**Step 3: Test with existing examples**

Run: `cargo run --example counter`
Expected: Runs without errors, displays counter UI correctly

**Step 4: Commit**

```bash
git add crates/sparsh/src/app.rs
git commit -m "feat(app): call paint_after_children hook after painting children"
```

---

## Task 5: Implement Translation Rendering in Shape Pass

**Files:**
- Modify: `crates/sparsh-render/src/shape_pass.rs:1-end`

**Step 1: Read current shape_pass implementation**

Run: `cat crates/sparsh-render/src/shape_pass.rs | head -100`
Expected: Understand how draw commands are processed

**Step 2: Add translation stack to ShapePass struct**

Find the ShapePass struct and add a translation stack field:

```rust
pub struct ShapePass {
    // ... existing fields ...
    translation_stack: Vec<(f32, f32)>,
}
```

**Step 3: Initialize translation stack in new()**

In the `new()` method, initialize:

```rust
translation_stack: vec![(0.0, 0.0)],
```

**Step 4: Update render() to handle translation commands**

In the command processing loop, add cases for PushTranslation and PopTranslation:

```rust
DrawCommand::PushTranslation { offset } => {
    let current = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
    self.translation_stack.push((current.0 + offset.0, current.1 + offset.1));
}
DrawCommand::PopTranslation => {
    if self.translation_stack.len() > 1 {
        self.translation_stack.pop();
    }
}
```

**Step 5: Apply translation to rect bounds**

When processing `DrawCommand::Rect`, apply the current translation:

Before adding to instance buffer, adjust bounds:

```rust
DrawCommand::Rect { bounds, color, corner_radius, border_width, border_color } => {
    let translation = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
    let translated_bounds = Rect::new(
        bounds.x + translation.0,
        bounds.y + translation.1,
        bounds.width,
        bounds.height,
    );
    // ... rest of existing rect processing with translated_bounds ...
}
```

**Step 6: Verify compilation**

Run: `cargo check -p sparsh-render`
Expected: SUCCESS with no errors

**Step 7: Commit**

```bash
git add crates/sparsh-render/src/shape_pass.rs
git commit -m "feat(render): implement translation stack in shape pass"
```

---

## Task 6: Implement Translation Rendering in Text Pass

**Files:**
- Modify: `crates/sparsh-render/src/text_pass.rs:1-end`

**Step 1: Read current text_pass implementation**

Run: `cat crates/sparsh-render/src/text_pass.rs | head -100`
Expected: Understand how text draw commands are processed

**Step 2: Add translation stack to TextPass struct**

Find the TextPass struct and add:

```rust
pub struct TextPass {
    // ... existing fields ...
    translation_stack: Vec<(f32, f32)>,
}
```

**Step 3: Initialize translation stack in new()**

In the `new()` method, initialize:

```rust
translation_stack: vec![(0.0, 0.0)],
```

**Step 4: Update render() to handle translation commands**

In the command processing loop, add cases:

```rust
DrawCommand::PushTranslation { offset } => {
    let current = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
    self.translation_stack.push((current.0 + offset.0, current.1 + offset.1));
}
DrawCommand::PopTranslation => {
    if self.translation_stack.len() > 1 {
        self.translation_stack.pop();
    }
}
```

**Step 5: Apply translation to glyph positions**

When processing `DrawCommand::Text`, apply translation to each glyph:

```rust
DrawCommand::Text { glyphs } => {
    let translation = self.translation_stack.last().copied().unwrap_or((0.0, 0.0));
    for glyph in glyphs {
        let translated_glyph = GlyphInstance {
            pos: [glyph.pos[0] + translation.0, glyph.pos[1] + translation.1],
            ..*glyph
        };
        // ... rest of existing glyph processing with translated_glyph ...
    }
}
```

**Step 6: Verify compilation**

Run: `cargo check -p sparsh-render`
Expected: SUCCESS with no errors

**Step 7: Commit**

```bash
git add crates/sparsh-render/src/text_pass.rs
git commit -m "feat(render): implement translation stack in text pass"
```

---

## Task 7: Update Scroll Widget to Use Translation

**Files:**
- Modify: `crates/sparsh-widgets/src/scroll.rs:150-250`

**Step 1: Update Scroll::paint() to push translation**

Find the `paint()` method implementation (around line 200-230). Replace the existing implementation with:

```rust
fn paint(&self, ctx: &mut PaintContext) {
    let bounds = ctx.bounds();

    // Draw scrollbar background (track)
    if self.direction == ScrollDirection::Vertical || self.direction == ScrollDirection::Both {
        let track_bounds = Rect::new(
            bounds.x + bounds.width - self.style.width * ctx.scale_factor,
            bounds.y,
            self.style.width * ctx.scale_factor,
            bounds.height,
        );
        ctx.fill_rounded_rect(
            track_bounds,
            self.style.track_color,
            self.style.corner_radius * ctx.scale_factor,
        );

        // Draw scrollbar thumb
        if self.content_size.1 > bounds.height / ctx.scale_factor {
            let viewport_ratio = (bounds.height / ctx.scale_factor) / self.content_size.1;
            let thumb_height = (bounds.height * viewport_ratio).max(20.0 * ctx.scale_factor);
            let scroll_ratio = self.offset_y / (self.content_size.1 - bounds.height / ctx.scale_factor).max(1.0);
            let thumb_y = bounds.y + scroll_ratio * (bounds.height - thumb_height);

            let thumb_bounds = Rect::new(
                track_bounds.x,
                thumb_y,
                track_bounds.width,
                thumb_height,
            );

            let thumb_color = if self.hover_scrollbar || self.dragging_scrollbar {
                self.style.thumb_hover_color
            } else {
                self.style.thumb_color
            };

            ctx.fill_rounded_rect(
                thumb_bounds,
                thumb_color,
                self.style.corner_radius * ctx.scale_factor,
            );
        }
    }

    // Clip to viewport (hide overflow)
    ctx.push_clip(bounds);

    // Translate content by negative scroll offset (physical pixels)
    let offset_x_physical = -self.offset_x * ctx.scale_factor;
    let offset_y_physical = -self.offset_y * ctx.scale_factor;
    ctx.push_translation((offset_x_physical, offset_y_physical));

    // Children will be painted by framework with translation active
}
```

**Step 2: Implement paint_after_children for Scroll**

Add the new method after the `paint()` method:

```rust
fn paint_after_children(&self, ctx: &mut PaintContext) {
    // Pop translation and clip in reverse order
    ctx.pop_translation();
    ctx.pop_clip();
}
```

**Step 3: Verify compilation**

Run: `cargo check -p sparsh-widgets`
Expected: SUCCESS with no errors

**Step 4: Commit**

```bash
git add crates/sparsh-widgets/src/scroll.rs
git commit -m "feat(scroll): implement content translation using paint hooks"
```

---

## Task 8: Fix Layout Example Syntax Errors

**Files:**
- Modify: `examples/layout/src/main.rs:1-248`

**Step 1: Read the current state**

Run: `cat examples/layout/src/main.rs | tail -20`
Expected: See the syntax errors that need fixing

**Step 2: Revert to proper Scroll wrapper usage**

Ensure the layout example uses Scroll::new().vertical().content(...) pattern correctly. The Container should be wrapped in Scroll, not replaced.

**Step 3: Verify compilation**

Run: `cargo check --example layout`
Expected: SUCCESS with no errors

**Step 4: Commit**

```bash
git add examples/layout/src/main.rs
git commit -m "fix(examples): restore proper Scroll usage in layout example"
```

---

## Task 9: Test Scrolling in Layout Example

**Files:**
- Test: `examples/layout/src/main.rs`

**Step 1: Run the layout example**

Run: `cargo run --example layout`
Expected: Window opens showing layout gallery

**Step 2: Test vertical scrolling**

Action: Scroll down with mouse wheel or trackpad
Expected: Content scrolls smoothly, revealing rows below the viewport

**Step 3: Verify clipping**

Expected: Content outside viewport is clipped (not visible)

**Step 4: Verify scrollbar**

Expected: Scrollbar appears on right side, thumb moves with scroll position

**Step 5: Test other examples**

Run: `cargo run --example kitchen-sink`
Expected: Scroll container in kitchen-sink works correctly

Run: `cargo run --example counter`
Expected: Still works (no scroll widget, unaffected by changes)

---

## Task 10: Manual Testing and Validation

**Step 1: Test edge cases**

- Scroll to top (offset = 0)
- Scroll to bottom (offset = max)
- Scroll with small content (no scrolling needed)
- Scroll with large content (multiple screens)

**Step 2: Verify no regressions**

Run all examples and verify they work:
- `cargo run --example triangle`
- `cargo run --example counter`
- `cargo run --example demo`
- `cargo run --example layout`
- `cargo run --example kitchen-sink`

**Step 3: Final commit**

If any issues found and fixed:

```bash
git add .
git commit -m "fix(scroll): address edge cases and testing feedback"
```

---

## Success Criteria

1. ✅ Translation draw commands added to rendering pipeline
2. ✅ paint_after_children hook added to Widget trait
3. ✅ Framework calls paint_after_children after painting children
4. ✅ ShapePass applies translation to rectangles
5. ✅ TextPass applies translation to text glyphs
6. ✅ Scroll widget uses translation to offset content
7. ✅ Layout example scrolls correctly
8. ✅ Kitchen-sink example scrolls correctly
9. ✅ No regressions in other examples
10. ✅ Clipping works correctly with translation

## Testing Notes

Since this is a rendering feature, testing is primarily manual/visual:
- Scroll wheel/trackpad input should translate content
- Content outside viewport should be clipped
- Scrollbar thumb should reflect scroll position
- All draw commands (rects, text) should be translated

No unit tests are needed for this feature as it's a visual/rendering behavior that requires actual GPU rendering to test properly.
