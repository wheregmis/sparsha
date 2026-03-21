# Rename Sikhar to Sparsh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Comprehensively rename all "sikhar" references to "sparsh" throughout the codebase

**Architecture:** This is a systematic renaming task affecting directory names, crate names, Cargo.toml files, import statements, and documentation. The rename must maintain consistency across the entire project.

**Tech Stack:** Rust, Cargo workspace

---

## Task 1: Rename Crate Directories

**Files:**
- Rename: `crates/sikhar` → `crates/sparsh`
- Rename: `crates/sikhar-core` → `crates/sparsh-core`
- Rename: `crates/sikhar-render` → `crates/sparsh-render`
- Rename: `crates/sikhar-layout` → `crates/sparsh-layout`
- Rename: `crates/sikhar-text` → `crates/sparsh-text`
- Rename: `crates/sikhar-input` → `crates/sparsh-input`
- Rename: `crates/sikhar-widgets` → `crates/sparsh-widgets`
- Rename: `crates/sikhar-native-apple` → `crates/sparsh-native-apple`

**Step 1: Rename all crate directories**

Run:
```bash
cd /Users/wheregmis/Documents/GitHub/sparsh/crates
mv sikhar sparsh
mv sikhar-core sparsh-core
mv sikhar-render sparsh-render
mv sikhar-layout sparsh-layout
mv sikhar-text sparsh-text
mv sikhar-input sparsh-input
mv sikhar-widgets sparsh-widgets
mv sikhar-native-apple sparsh-native-apple
```

Expected: All directories renamed successfully

**Step 2: Verify directory structure**

Run: `ls -la /Users/wheregmis/Documents/GitHub/sparsh/crates`

Expected: See sparsh, sparsh-core, sparsh-render, sparsh-layout, sparsh-text, sparsh-input, sparsh-widgets, sparsh-native-apple

---

## Task 2: Update Root Cargo.toml

**Files:**
- Modify: `Cargo.toml`

**Step 1: Update workspace members**

Replace in `Cargo.toml`:
```toml
members = [
    "crates/sparsh-core",
    "crates/sparsh-render",
    "crates/sparsh-layout",
    "crates/sparsh-text",
    "crates/sparsh-input",
    "crates/sparsh-widgets",
    "crates/sparsh-native-apple",
    "crates/sparsh",
    "examples/triangle",
    "examples/demo",
    "examples/counter",
    "examples/native-demo",
    "run-wasm",
]
```

**Step 2: Update workspace dependencies**

Replace in `Cargo.toml`:
```toml
# Internal crates
sparsh-core = { path = "crates/sparsh-core" }
sparsh-render = { path = "crates/sparsh-render" }
sparsh-layout = { path = "crates/sparsh-layout" }
sparsh-text = { path = "crates/sparsh-text" }
sparsh-input = { path = "crates/sparsh-input" }
sparsh-widgets = { path = "crates/sparsh-widgets" }
sparsh-native-apple = { path = "crates/sparsh-native-apple" }
```

**Step 3: Verify Cargo.toml**

Run: `cat Cargo.toml | grep -E "(sparsh|sikhar)"`

Expected: Only "sparsh" references, no "sikhar"

---

## Task 3: Update sparsh-core Crate

**Files:**
- Modify: `crates/sparsh-core/Cargo.toml`

**Step 1: Update crate name in Cargo.toml**

Replace `name = "sikhar-core"` with `name = "sparsh-core"`

**Step 2: Verify**

Run: `cat crates/sparsh-core/Cargo.toml | grep name`

Expected: `name = "sparsh-core"`

---

## Task 4: Update sparsh-render Crate

**Files:**
- Modify: `crates/sparsh-render/Cargo.toml`
- Modify: `crates/sparsh-render/src/lib.rs`
- Modify: `crates/sparsh-render/src/renderer.rs`
- Modify: `crates/sparsh-render/src/shape_pass.rs`
- Modify: `crates/sparsh-render/src/text_pass.rs`
- Modify: `crates/sparsh-render/src/commands.rs`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar-render"` with `name = "sparsh-render"`
Replace `sikhar-core` with `sparsh-core` in dependencies

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core` in all files
Replace all `sikhar_core::` with `sparsh_core::` in all files

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh-render/src/`

Expected: No matches

---

## Task 5: Update sparsh-layout Crate

**Files:**
- Modify: `crates/sparsh-layout/Cargo.toml`
- Modify: `crates/sparsh-layout/src/lib.rs`
- Modify: `crates/sparsh-layout/src/tree.rs`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar-layout"` with `name = "sparsh-layout"`
Replace `sikhar-core` with `sparsh-core` in dependencies

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core`
Replace all `sikhar_core::` with `sparsh_core::`

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh-layout/src/`

Expected: No matches

---

## Task 6: Update sparsh-text Crate

**Files:**
- Modify: `crates/sparsh-text/Cargo.toml`
- Modify: `crates/sparsh-text/src/lib.rs`
- Modify: `crates/sparsh-text/src/system.rs`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar-text"` with `name = "sparsh-text"`
Replace `sikhar-core` and `sikhar-render` with `sparsh-core` and `sparsh-render` in dependencies

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core`
Replace all `use sikhar_render` with `use sparsh_render`
Replace all `sikhar_core::` with `sparsh_core::`
Replace all `sikhar_render::` with `sparsh_render::`

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh-text/src/`

Expected: No matches

---

## Task 7: Update sparsh-input Crate

**Files:**
- Modify: `crates/sparsh-input/Cargo.toml`
- Modify: `crates/sparsh-input/src/lib.rs`
- Modify: `crates/sparsh-input/src/focus.rs`
- Modify: `crates/sparsh-input/src/hit_test.rs`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar-input"` with `name = "sparsh-input"`
Replace `sikhar-core` with `sparsh-core` in dependencies

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core`
Replace all `sikhar_core::` with `sparsh_core::`

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh-input/src/`

Expected: No matches

---

## Task 8: Update sparsh-widgets Crate

**Files:**
- Modify: `crates/sparsh-widgets/Cargo.toml`
- Modify: `crates/sparsh-widgets/src/lib.rs`
- Modify: `crates/sparsh-widgets/src/widget.rs`
- Modify: `crates/sparsh-widgets/src/context.rs`
- Modify: `crates/sparsh-widgets/src/button.rs`
- Modify: `crates/sparsh-widgets/src/container.rs`
- Modify: `crates/sparsh-widgets/src/text.rs`
- Modify: `crates/sparsh-widgets/src/text_input.rs`
- Modify: `crates/sparsh-widgets/src/scroll.rs`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar-widgets"` with `name = "sparsh-widgets"`
Replace all `sikhar-*` dependencies with `sparsh-*`

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core`
Replace all `use sikhar_render` with `use sparsh_render`
Replace all `use sikhar_layout` with `use sparsh_layout`
Replace all `use sikhar_text` with `use sparsh_text`
Replace all `use sikhar_input` with `use sparsh_input`
Replace all `sikhar_*::` with `sparsh_*::`

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh-widgets/src/`

Expected: No matches

---

## Task 9: Update sparsh-native-apple Crate

**Files:**
- Modify: `crates/sparsh-native-apple/Cargo.toml`
- Modify: `crates/sparsh-native-apple/src/lib.rs`
- Modify: `crates/sparsh-native-apple/src/native_widget.rs`
- Modify: `crates/sparsh-native-apple/src/view_manager.rs`
- Modify: `crates/sparsh-native-apple/src/layout.rs`
- Modify: `crates/sparsh-native-apple/src/events.rs`
- Modify: All widget files in `crates/sparsh-native-apple/src/widgets/`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar-native-apple"` with `name = "sparsh-native-apple"`
Replace all `sikhar-*` dependencies with `sparsh-*`

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core`
Replace all `use sikhar_render` with `use sparsh_render`
Replace all `use sikhar_layout` with `use sparsh_layout`
Replace all `use sikhar_widgets` with `use sparsh_widgets`
Replace all `sikhar_*::` with `sparsh_*::`

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh-native-apple/src/`

Expected: No matches

---

## Task 10: Update sparsh (Main) Crate

**Files:**
- Modify: `crates/sparsh/Cargo.toml`
- Modify: `crates/sparsh/src/lib.rs`
- Modify: `crates/sparsh/src/app.rs`
- Modify: `crates/sparsh/src/accessibility.rs`
- Modify: `crates/sparsh/src/web.rs`

**Step 1: Update Cargo.toml**

Replace `name = "sikhar"` with `name = "sparsh"`
Replace all `sikhar-*` dependencies with `sparsh-*`

**Step 2: Update import statements**

Replace all `use sikhar_core` with `use sparsh_core`
Replace all `use sikhar_render` with `use sparsh_render`
Replace all `use sikhar_layout` with `use sparsh_layout`
Replace all `use sikhar_text` with `use sparsh_text`
Replace all `use sikhar_input` with `use sparsh_input`
Replace all `use sikhar_widgets` with `use sparsh_widgets`
Replace all `sikhar_*::` with `sparsh_*::`

**Step 3: Verify**

Run: `grep -r "sikhar" crates/sparsh/src/`

Expected: No matches

---

## Task 11: Update Examples

**Files:**
- Modify: `examples/triangle/Cargo.toml`
- Modify: `examples/triangle/src/main.rs`
- Modify: `examples/counter/Cargo.toml`
- Modify: `examples/counter/src/lib.rs`
- Modify: `examples/demo/Cargo.toml`
- Modify: `examples/demo/src/main.rs`
- Modify: `examples/native-demo/Cargo.toml`
- Modify: `examples/native-demo/src/main.rs`

**Step 1: Update all example Cargo.toml files**

Replace `sikhar` dependency with `sparsh` in dependencies section

**Step 2: Update all example source files**

Replace all `use sikhar::` with `use sparsh::`
Replace all `sikhar::` with `sparsh::`

**Step 3: Verify**

Run: `grep -r "sikhar" examples/`

Expected: No matches

---

## Task 12: Update run-wasm

**Files:**
- Modify: `run-wasm/Cargo.toml`

**Step 1: Update Cargo.toml**

Replace `sikhar` dependency with `sparsh`

**Step 2: Verify**

Run: `grep "sikhar" run-wasm/Cargo.toml`

Expected: No matches

---

## Task 13: Update README.md

**Files:**
- Modify: `README.md`

**Step 1: Update title and references**

Replace:
- `# Sikhar` with `# Sparsh`
- All `sikhar` with `sparsh` in text
- All `sikhar-*` crate names with `sparsh-*`
- Update example code from `use sikhar::prelude::*;` to `use sparsh::prelude::*;`

**Step 2: Verify**

Run: `grep -i "sikhar" README.md`

Expected: No matches

---

## Task 14: Update Documentation Files

**Files:**
- Modify: `.cursor/plans/sikhar-ui-framework-a03eb39e.plan.md` (if needed for historical purposes)

**Step 1: Check and update plan files**

Review plan files and update references if necessary for clarity

**Step 2: Verify**

Run: `find docs -type f -name "*.md" -exec grep -l "sikhar" {} \;`

Expected: No critical references (old plans can retain historical names)

---

## Task 15: Clean Build Artifacts

**Files:**
- Remove: `target/` directory contents

**Step 1: Clean cargo build**

Run: `cargo clean`

Expected: Build artifacts removed

**Step 2: Verify clean state**

Run: `ls target/ | wc -l`

Expected: Minimal entries (0 or just a few directories)

---

## Task 16: Build and Test

**Step 1: Build all crates**

Run: `cargo build --workspace`

Expected: Successful build with no errors
```
   Compiling sparsh-core v0.1.0
   Compiling sparsh-render v0.1.0
   Compiling sparsh-layout v0.1.0
   ...
   Finished dev [unoptimized + debuginfo] target(s)
```

**Step 2: Build examples**

Run: `cargo build --examples`

Expected: All examples build successfully

**Step 3: Run triangle example**

Run: `cargo run -p triangle`

Expected: Triangle example window opens and displays correctly

**Step 4: Run demo example**

Run: `cargo run -p demo`

Expected: Demo application runs without errors

---

## Task 17: Commit Changes

**Step 1: Stage all changes**

Run:
```bash
git add -A
```

**Step 2: Review changes**

Run: `git status`

Expected: All renamed files and modified files staged

**Step 3: Commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
refactor: rename sikhar to sparsh throughout codebase

- Rename all crate directories from sikhar-* to sparsh-*
- Update all Cargo.toml files with new crate names
- Update all import statements and references
- Update README.md and documentation
- Verified all examples build and run successfully

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
EOF
)"
```

Expected: Commit created successfully

**Step 4: Verify commit**

Run: `git log -1 --stat`

Expected: See commit with all changed files

---

## Notes

- This is a comprehensive rename affecting 8 crates, 4 examples, and documentation
- All internal references must be updated to maintain build consistency
- The workspace structure remains the same, only names change
- Building after rename verifies all references are correctly updated
- No functionality changes, only naming changes
