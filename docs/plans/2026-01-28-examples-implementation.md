# Examples Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Layout Gallery and Kitchen Sink examples for comprehensive testing of layout, rendering, and hit testing

**Architecture:** Create two new focused examples: Layout Gallery (visual layout testing) and Kitchen Sink (interactive widget testing). Remove run-wasm directory and update workspace configuration. Keep existing examples for reference.

**Tech Stack:** Rust, Sparsh UI framework, cargo workspace

---

## Task 1: Remove run-wasm Directory

**Files:**
- Remove: `run-wasm/` directory
- Modify: `Cargo.toml`

**Step 1: Remove run-wasm directory**

Run:
```bash
rm -rf run-wasm
```

Expected: Directory removed

**Step 2: Update workspace members in root Cargo.toml**

Edit `Cargo.toml`:
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
    "examples/layout",
    "examples/kitchen-sink",
]
```

**Step 3: Verify workspace builds**

Run: `cargo check --workspace`
Expected: Success

**Step 4: Commit**

```bash
git add -A
git commit -m "refactor: remove run-wasm directory and update workspace"
```

---

## Task 2: Create Layout Gallery Example Structure

**Files:**
- Create: `examples/layout/Cargo.toml`
- Create: `examples/layout/src/main.rs`

**Step 1: Create directory structure**

Run:
```bash
mkdir -p examples/layout/src
```

Expected: Directory created

**Step 2: Create Cargo.toml**

Create `examples/layout/Cargo.toml`:
```toml
[package]
name = "layout"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "layout"
path = "src/main.rs"

[dependencies]
sparsh = { workspace = true }
env_logger = "0.11"
log = "0.4"
```

**Step 3: Create minimal main.rs**

Create `examples/layout/src/main.rs`:
```rust
//! Layout Gallery - Visual testing for layout features

use sparsh::prelude::*;

fn main() {
    env_logger::init();

    App::new()
        .with_title("Layout Gallery - Sparsh")
        .with_size(1000, 800)
        .with_background(Color::from_hex(0x1F2937))
        .run(build_ui);
}

fn build_ui() -> Box<dyn Widget> {
    Box::new(
        Container::new()
            .fill()
            .background(Color::from_hex(0x1F2937))
            .child(
                Text::new("Layout Gallery")
                    .size(24.0)
                    .bold()
                    .color(Color::WHITE),
            ),
    )
}
```

**Step 4: Build and verify**

Run: `cargo build -p layout`
Expected: Builds successfully

**Step 5: Test run**

Run: `cargo run -p layout`
Expected: Window opens with title

**Step 6: Commit**

```bash
git add examples/layout
git commit -m "feat: add layout gallery example skeleton"
```

---

## Task 3: Implement Layout Gallery Test Sections

**Files:**
- Modify: `examples/layout/src/main.rs`

**Step 1: Create test section helper**

Add to `examples/layout/src/main.rs`:
```rust
/// Creates a labeled test section
fn test_section(title: &str, content: Container) -> Container {
    Container::new()
        .column()
        .gap(8.0)
        .padding(16.0)
        .background(Color::from_hex(0x374151))
        .corner_radius(8.0)
        .min_size(300.0, 0.0)
        .child(
            Text::new(title)
                .size(14.0)
                .bold()
                .color(Color::from_hex(0xF3F4F6)),
        )
        .child(content)
}

/// Creates a colored test box
fn color_box(hex: u32, width: f32, height: f32) -> Container {
    Container::new()
        .size(width, height)
        .background(Color::from_hex(hex))
        .corner_radius(4.0)
}
```

**Step 2: Implement Row 1 - Flex Direction tests**

Update `build_ui()` function:
```rust
fn build_ui() -> Box<dyn Widget> {
    Box::new(
        Scroll::new(
            Container::new()
                .column()
                .gap(24.0)
                .padding(32.0)
                .background(Color::from_hex(0x1F2937))
                // Header
                .child(
                    Text::new("Layout Gallery")
                        .size(32.0)
                        .bold()
                        .color(Color::WHITE),
                )
                // Row 1: Flex Direction
                .child(
                    Container::new()
                        .row()
                        .gap(16.0)
                        .child(test_section(
                            "Column Layout",
                            Container::new()
                                .column()
                                .gap(8.0)
                                .child(color_box(0xEF4444, 60.0, 40.0)) // Red
                                .child(color_box(0x22C55E, 60.0, 40.0)) // Green
                                .child(color_box(0x3B82F6, 60.0, 40.0)), // Blue
                        ))
                        .child(test_section(
                            "Row Layout",
                            Container::new()
                                .row()
                                .gap(8.0)
                                .child(color_box(0xEF4444, 60.0, 60.0))
                                .child(color_box(0x22C55E, 60.0, 60.0))
                                .child(color_box(0x3B82F6, 60.0, 60.0)),
                        ))
                        .child(test_section(
                            "Wrap Layout",
                            Container::new()
                                .row()
                                .wrap()
                                .gap(8.0)
                                .max_size(200.0, 0.0)
                                .child(color_box(0xEF4444, 60.0, 40.0))
                                .child(color_box(0xF59E0B, 60.0, 40.0))
                                .child(color_box(0x22C55E, 60.0, 40.0))
                                .child(color_box(0x3B82F6, 60.0, 40.0))
                                .child(color_box(0x8B5CF6, 60.0, 40.0))
                                .child(color_box(0xEC4899, 60.0, 40.0)),
                        )),
                ),
        ).vertical(),
    )
}
```

**Step 3: Add Row 2 - Alignment tests**

Add after Row 1:
```rust
// Row 2: Alignment
.child(
    Container::new()
        .row()
        .gap(16.0)
        .child(test_section(
            "Align Start",
            Container::new()
                .column()
                .align_start()
                .min_size(120.0, 150.0)
                .background(Color::from_hex(0x1F2937))
                .child(color_box(0xEF4444, 60.0, 40.0))
                .child(color_box(0x22C55E, 60.0, 40.0)),
        ))
        .child(test_section(
            "Align Center",
            Container::new()
                .column()
                .center()
                .min_size(120.0, 150.0)
                .background(Color::from_hex(0x1F2937))
                .child(color_box(0xEF4444, 60.0, 40.0))
                .child(color_box(0x22C55E, 60.0, 40.0)),
        ))
        .child(test_section(
            "Align End",
            Container::new()
                .column()
                .align_end()
                .min_size(120.0, 150.0)
                .background(Color::from_hex(0x1F2937))
                .child(color_box(0xEF4444, 60.0, 40.0))
                .child(color_box(0x22C55E, 60.0, 40.0)),
        )),
)
```

**Step 4: Add Row 3 - Spacing tests**

Add after Row 2:
```rust
// Row 3: Spacing
.child(
    Container::new()
        .row()
        .gap(16.0)
        .child(test_section(
            "No Gap",
            Container::new()
                .row()
                .gap(0.0)
                .child(color_box(0xEF4444, 40.0, 40.0))
                .child(color_box(0x22C55E, 40.0, 40.0))
                .child(color_box(0x3B82F6, 40.0, 40.0)),
        ))
        .child(test_section(
            "Gap 8px",
            Container::new()
                .row()
                .gap(8.0)
                .child(color_box(0xEF4444, 40.0, 40.0))
                .child(color_box(0x22C55E, 40.0, 40.0))
                .child(color_box(0x3B82F6, 40.0, 40.0)),
        ))
        .child(test_section(
            "Gap 24px",
            Container::new()
                .row()
                .gap(24.0)
                .child(color_box(0xEF4444, 40.0, 40.0))
                .child(color_box(0x22C55E, 40.0, 40.0))
                .child(color_box(0x3B82F6, 40.0, 40.0)),
        )),
)
```

**Step 5: Add Row 4 - Sizing tests**

Add after Row 3:
```rust
// Row 4: Sizing
.child(
    Container::new()
        .row()
        .gap(16.0)
        .child(test_section(
            "Fixed Size",
            Container::new()
                .row()
                .gap(8.0)
                .child(color_box(0xEF4444, 100.0, 100.0))
                .child(color_box(0x22C55E, 100.0, 100.0)),
        ))
        .child(test_section(
            "Flex Grow",
            Container::new()
                .row()
                .gap(8.0)
                .min_size(250.0, 0.0)
                .child(
                    Container::new()
                        .flex_grow(1.0)
                        .min_size(0.0, 60.0)
                        .background(Color::from_hex(0xEF4444))
                        .corner_radius(4.0),
                )
                .child(
                    Container::new()
                        .flex_grow(1.0)
                        .min_size(0.0, 60.0)
                        .background(Color::from_hex(0x22C55E))
                        .corner_radius(4.0),
                ),
        ))
        .child(test_section(
            "Min/Max Size",
            Container::new()
                .column()
                .gap(8.0)
                .child(
                    Container::new()
                        .min_size(80.0, 40.0)
                        .max_size(120.0, 40.0)
                        .background(Color::from_hex(0xEF4444))
                        .corner_radius(4.0),
                )
                .child(
                    Container::new()
                        .min_size(60.0, 40.0)
                        .max_size(100.0, 40.0)
                        .background(Color::from_hex(0x22C55E))
                        .corner_radius(4.0),
                ),
        )),
)
```

**Step 6: Add Row 5 - Nesting tests**

Add after Row 4:
```rust
// Row 5: Nesting
.child(
    test_section(
        "Nested Containers (3 levels)",
        Container::new()
            .padding(16.0)
            .background(Color::from_hex(0x3B82F6)) // Blue
            .corner_radius(8.0)
            .child(
                Container::new()
                    .padding(16.0)
                    .background(Color::from_hex(0x22C55E)) // Green
                    .corner_radius(8.0)
                    .child(
                        Container::new()
                            .padding(16.0)
                            .background(Color::from_hex(0x8B5CF6)) // Purple
                            .corner_radius(8.0)
                            .child(
                                Text::new("Level 3")
                                    .size(14.0)
                                    .color(Color::WHITE),
                            ),
                    ),
            ),
    ),
)
```

**Step 7: Build and test**

Run: `cargo run -p layout`
Expected: Window shows all test sections

**Step 8: Commit**

```bash
git add examples/layout/src/main.rs
git commit -m "feat: implement all layout gallery test sections"
```

---

## Task 4: Create Kitchen Sink Example Structure

**Files:**
- Create: `examples/kitchen-sink/Cargo.toml`
- Create: `examples/kitchen-sink/src/main.rs`

**Step 1: Create directory structure**

Run:
```bash
mkdir -p examples/kitchen-sink/src
```

Expected: Directory created

**Step 2: Create Cargo.toml**

Create `examples/kitchen-sink/Cargo.toml`:
```toml
[package]
name = "kitchen-sink"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "kitchen-sink"
path = "src/main.rs"

[dependencies]
sparsh = { workspace = true }
env_logger = "0.11"
log = "0.4"
```

**Step 3: Create basic structure with state**

Create `examples/kitchen-sink/src/main.rs`:
```rust
//! Kitchen Sink - Interactive widget testing

use sparsh::prelude::*;
use sparsh::widgets::{LayoutContext, PaintContext, Widget, WidgetId};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

fn main() {
    env_logger::init();

    App::new()
        .with_title("Kitchen Sink - Sparsh")
        .with_size(1200, 900)
        .with_background(Color::from_hex(0x0F172A))
        .run(|| Box::new(KitchenSinkApp::new()));
}

/// Shared state for tracking interactions
#[derive(Clone)]
struct AppState {
    hover_count: Arc<AtomicU32>,
    click_count: Arc<AtomicU32>,
}

impl AppState {
    fn new() -> Self {
        Self {
            hover_count: Arc::new(AtomicU32::new(0)),
            click_count: Arc::new(AtomicU32::new(0)),
        }
    }
}

struct KitchenSinkApp {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl KitchenSinkApp::new() -> Self {
    let state = AppState::new();

    Self {
        id: WidgetId::default(),
        children: vec![
            // Will add sections here
        ],
    }
}

impl Widget for KitchenSinkApp {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> sparsh::layout::taffy::Style {
        use sparsh::layout::taffy::prelude::*;
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        ctx.fill_rect(ctx.bounds(), Color::from_hex(0x0F172A));
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}
```

**Step 4: Build and verify**

Run: `cargo build -p kitchen-sink`
Expected: Builds successfully

**Step 5: Commit**

```bash
git add examples/kitchen-sink
git commit -m "feat: add kitchen-sink example skeleton"
```

---

## Task 5: Implement Kitchen Sink Sidebar

**Files:**
- Modify: `examples/kitchen-sink/src/main.rs`

**Step 1: Create sidebar component**

Add to `examples/kitchen-sink/src/main.rs`:
```rust
/// Left sidebar with button gallery and text samples
struct Sidebar {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl Sidebar {
    fn new(state: AppState) -> Self {
        let click_state = state.clone();

        Self {
            id: WidgetId::default(),
            children: vec![
                // Title
                Box::new(
                    Text::new("Kitchen Sink")
                        .size(20.0)
                        .bold()
                        .color(Color::WHITE),
                ),
                // Section: Button Gallery
                Box::new(
                    Text::new("Buttons")
                        .size(16.0)
                        .bold()
                        .color(Color::from_hex(0x94A3B8)),
                ),
                Box::new(
                    Button::new("Default")
                        .background(Color::from_hex(0x3B82F6))
                        .on_click(move || {
                            click_state.click_count.fetch_add(1, Ordering::Relaxed);
                        }),
                ),
                Box::new(
                    Button::new("Success")
                        .background(Color::from_hex(0x22C55E))
                        .on_click(|| {}),
                ),
                Box::new(
                    Button::new("Danger")
                        .background(Color::from_hex(0xEF4444))
                        .on_click(|| {}),
                ),
                Box::new(
                    Button::new("Warning")
                        .background(Color::from_hex(0xF59E0B))
                        .on_click(|| {}),
                ),
                Box::new(
                    Button::new("Secondary")
                        .background(Color::from_hex(0x64748B))
                        .on_click(|| {}),
                ),
                // Section: Typography
                Box::new(
                    Text::new("Typography")
                        .size(16.0)
                        .bold()
                        .color(Color::from_hex(0x94A3B8)),
                ),
                Box::new(
                    Text::new("Heading Text")
                        .size(24.0)
                        .bold()
                        .color(Color::WHITE),
                ),
                Box::new(
                    Text::new("Body text example")
                        .size(16.0)
                        .color(Color::from_hex(0xE2E8F0)),
                ),
                Box::new(
                    Text::new("Small caption")
                        .size(12.0)
                        .color(Color::from_hex(0x94A3B8)),
                ),
            ],
        }
    }
}

impl Widget for Sidebar {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> sparsh::layout::taffy::Style {
        use sparsh::layout::taffy::prelude::*;
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: length(220.0),
                height: percent(1.0),
            },
            gap: Size {
                width: length(0.0),
                height: length(12.0),
            },
            padding: Rect::from_length(24.0, 20.0, 24.0, 20.0),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        ctx.fill_rect(ctx.bounds(), Color::from_hex(0x1E293B));
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}
```

**Step 2: Update KitchenSinkApp to include sidebar**

Update `KitchenSinkApp::new()`:
```rust
impl KitchenSinkApp {
    fn new() -> Self {
        let state = AppState::new();

        Self {
            id: WidgetId::default(),
            children: vec![
                Box::new(Sidebar::new(state.clone())),
                // Main area will go here
            ],
        }
    }
}
```

**Step 3: Build and test**

Run: `cargo run -p kitchen-sink`
Expected: Window shows sidebar with buttons

**Step 4: Commit**

```bash
git add examples/kitchen-sink/src/main.rs
git commit -m "feat: implement kitchen-sink sidebar with buttons"
```

---

## Task 6: Implement Kitchen Sink Main Area

**Files:**
- Modify: `examples/kitchen-sink/src/main.rs`

**Step 1: Create main area component**

Add to `examples/kitchen-sink/src/main.rs`:
```rust
/// Main content area with scrollable sections
struct MainArea {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl MainArea {
    fn new(state: AppState) -> Self {
        Self {
            id: WidgetId::default(),
            children: vec![
                Box::new(InputSection::new()),
                Box::new(ContainerSection::new()),
                Box::new(ScrollSection::new()),
            ],
        }
    }
}

impl Widget for MainArea {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> sparsh::layout::taffy::Style {
        use sparsh::layout::taffy::prelude::*;
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            size: Size {
                width: auto(),
                height: percent(1.0),
            },
            gap: Size {
                width: length(0.0),
                height: length(32.0),
            },
            padding: Rect::from_length(32.0),
            overflow: Point {
                x: Overflow::Visible,
                y: Overflow::Scroll,
            },
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}
```

**Step 2: Create section helper**

Add helper:
```rust
/// Creates a labeled section container
fn section(title: &str, children: Vec<Box<dyn Widget>>) -> Container {
    Container::new()
        .column()
        .gap(16.0)
        .padding(24.0)
        .background(Color::from_hex(0x1E293B))
        .corner_radius(12.0)
        .child(
            Text::new(title)
                .size(18.0)
                .bold()
                .color(Color::WHITE),
        )
        .children(children)
}
```

**Step 3: Implement Input Section**

Add:
```rust
struct InputSection {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl InputSection {
    fn new() -> Self {
        Self {
            id: WidgetId::default(),
            children: vec![
                Box::new(
                    Text::new("Input Fields")
                        .size(18.0)
                        .bold()
                        .color(Color::WHITE),
                ),
                Box::new(
                    TextInput::new()
                        .placeholder("Enter text..."),
                ),
                Box::new(
                    TextInput::new()
                        .placeholder("Email address..."),
                ),
                Box::new(
                    TextInput::new()
                        .placeholder("Password..."),
                ),
            ],
        }
    }
}

impl Widget for InputSection {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> sparsh::layout::taffy::Style {
        use sparsh::layout::taffy::prelude::*;
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(12.0),
            },
            padding: Rect::from_length(24.0),
            background: Color::from_hex(0x1E293B),
            border_radius: Rect::from_length(12.0),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        ctx.fill_rounded_rect(ctx.bounds(), Color::from_hex(0x1E293B), 12.0);
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}
```

**Step 4: Implement Container Section**

Add:
```rust
struct ContainerSection {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl ContainerSection {
    fn new() -> Self {
        Self {
            id: WidgetId::default(),
            children: vec![
                Box::new(
                    Text::new("Nested Containers")
                        .size(18.0)
                        .bold()
                        .color(Color::WHITE),
                ),
                // 3-level nesting
                Box::new(
                    Container::new()
                        .padding(16.0)
                        .background(Color::from_hex(0x3B82F6).with_alpha(0.3))
                        .corner_radius(8.0)
                        .child(
                            Container::new()
                                .padding(16.0)
                                .background(Color::from_hex(0x22C55E).with_alpha(0.3))
                                .corner_radius(8.0)
                                .child(
                                    Container::new()
                                        .padding(16.0)
                                        .background(Color::from_hex(0x8B5CF6).with_alpha(0.3))
                                        .corner_radius(8.0)
                                        .child(
                                            Text::new("Level 3")
                                                .size(14.0)
                                                .color(Color::WHITE),
                                        ),
                                ),
                        ),
                ),
                Box::new(
                    Text::new("Overlapping Containers")
                        .size(16.0)
                        .bold()
                        .color(Color::from_hex(0x94A3B8)),
                ),
                // Overlapping containers
                Box::new(
                    Container::new()
                        .relative()
                        .min_size(300.0, 200.0)
                        .child(
                            Container::new()
                                .absolute()
                                .position(0.0, 0.0)
                                .size(150.0, 150.0)
                                .background(Color::from_hex(0xEF4444).with_alpha(0.7))
                                .corner_radius(8.0),
                        )
                        .child(
                            Container::new()
                                .absolute()
                                .position(50.0, 50.0)
                                .size(150.0, 150.0)
                                .background(Color::from_hex(0x3B82F6).with_alpha(0.7))
                                .corner_radius(8.0),
                        ),
                ),
            ],
        }
    }
}

impl Widget for ContainerSection {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> sparsh::layout::taffy::Style {
        use sparsh::layout::taffy::prelude::*;
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(16.0),
            },
            padding: Rect::from_length(24.0),
            background: Color::from_hex(0x1E293B),
            border_radius: Rect::from_length(12.0),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        ctx.fill_rounded_rect(ctx.bounds(), Color::from_hex(0x1E293B), 12.0);
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}
```

**Step 5: Implement Scroll Section**

Add:
```rust
struct ScrollSection {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl ScrollSection {
    fn new() -> Self {
        let mut items = Vec::new();
        for i in 0..20 {
            items.push(Box::new(
                Container::new()
                    .min_size(0.0, 40.0)
                    .background(if i % 2 == 0 {
                        Color::from_hex(0x334155)
                    } else {
                        Color::from_hex(0x1E293B)
                    })
                    .corner_radius(4.0)
                    .child(
                        Text::new(&format!("Item {}", i + 1))
                            .size(14.0)
                            .color(Color::WHITE),
                    ),
            ) as Box<dyn Widget>);
        }

        Self {
            id: WidgetId::default(),
            children: vec![
                Box::new(
                    Text::new("Scrollable Area")
                        .size(18.0)
                        .bold()
                        .color(Color::WHITE),
                ),
                Box::new(
                    Scroll::new(
                        Container::new()
                            .column()
                            .gap(8.0)
                            .children(items),
                    )
                    .vertical()
                    .max_size(0.0, 300.0),
                ),
            ],
        }
    }
}

impl Widget for ScrollSection {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> sparsh::layout::taffy::Style {
        use sparsh::layout::taffy::prelude::*;
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(16.0),
            },
            padding: Rect::from_length(24.0),
            background: Color::from_hex(0x1E293B),
            border_radius: Rect::from_length(12.0),
            ..Default::default()
        }
    }

    fn paint(&self, ctx: &mut PaintContext) {
        ctx.fill_rounded_rect(ctx.bounds(), Color::from_hex(0x1E293B), 12.0);
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}
```

**Step 6: Update KitchenSinkApp to include main area**

Update `KitchenSinkApp::new()`:
```rust
impl KitchenSinkApp {
    fn new() -> Self {
        let state = AppState::new();

        Self {
            id: WidgetId::default(),
            children: vec![
                Box::new(Sidebar::new(state.clone())),
                Box::new(MainArea::new(state)),
            ],
        }
    }
}
```

**Step 7: Build and test**

Run: `cargo run -p kitchen-sink`
Expected: Full UI with sidebar and main sections

**Step 8: Commit**

```bash
git add examples/kitchen-sink/src/main.rs
git commit -m "feat: implement kitchen-sink main area with sections"
```

---

## Task 7: Update Examples README

**Files:**
- Modify: `examples/README.md`

**Step 1: Update README content**

Edit `examples/README.md`:
```markdown
# Sparsh Examples

A collection of examples demonstrating the Sparsh UI framework.

## Examples

### Layout Gallery (`examples/layout`)
Visual testing ground for layout features including flex direction, alignment, spacing, sizing, and nesting.

**Run:**
```bash
cargo run -p layout
```

**Tests:**
- Column, row, and wrap layouts
- Start, center, and end alignment
- Gap spacing variations
- Fixed, flex-grow, and constrained sizing
- Nested container hierarchy

### Kitchen Sink (`examples/kitchen-sink`)
Comprehensive interactive example demonstrating all widgets and testing hit detection.

**Run:**
```bash
cargo run -p kitchen-sink
```

**Features:**
- Button gallery with different states and colors
- Typography samples
- Text input fields with focus management
- Nested and overlapping containers
- Scrollable content areas
- Interactive state tracking

### Triangle (`examples/triangle`)
Minimal GPU rendering test - a single colored triangle.

**Run:**
```bash
cargo run -p triangle
```

### Counter (`examples/counter`)
Stateful widget example with increment/decrement controls.

**Run:**
```bash
cargo run -p counter
```

### Demo (`examples/demo`)
Dashboard-style UI demonstrating complex layouts.

**Run:**
```bash
cargo run -p demo
```

### Native Demo (`examples/native-demo`)
Native macOS widget integration examples.

**Run:**
```bash
cargo run -p native-demo --features native
```

## Building All Examples

```bash
cargo build --examples
```

## Release Builds

For better performance:

```bash
cargo run -p layout --release
cargo run -p kitchen-sink --release
```
```

**Step 2: Commit**

```bash
git add examples/README.md
git commit -m "docs: update examples README with new examples"
```

---

## Task 8: Final Build and Test

**Step 1: Clean build**

Run: `cargo clean`
Expected: Target directory cleaned

**Step 2: Build all examples**

Run: `cargo build --examples`
Expected: All examples build successfully

**Step 3: Test layout example**

Run: `cargo run -p layout`
Expected: Window opens with all test sections visible and correctly laid out

**Step 4: Test kitchen-sink example**

Run: `cargo run -p kitchen-sink`
Expected: Window opens with sidebar, buttons respond to clicks, text inputs are focusable, scroll works

**Step 5: Verify workspace**

Run: `cargo check --workspace`
Expected: No errors

**Step 6: Final commit**

```bash
git add -A
git commit -m "feat: complete examples redesign with layout and kitchen-sink"
```

---

## Notes

- The examples use the existing Sparsh widget API
- Layout Gallery focuses on visual layout testing
- Kitchen Sink tests interactive features and hit detection
- Both examples use dark themes for consistency
- Code is organized with helper functions for reusability
- All sections are clearly labeled for easy debugging

## Future Enhancements

- Add performance metrics display in Kitchen Sink
- Add interactive layout parameter controls in Layout Gallery
- Add visual indicators for hover/focus states
- Add FPS counter in status bar
- Add more complex nested scenarios
