# Examples Redesign

## Goal

Create comprehensive, focused examples for testing layout, rendering, and hit testing in Sparsh. Remove web-specific code and focus on desktop functionality in alpha stage.

## Overview

Replace existing examples with two purpose-built testing examples:
1. **Layout Gallery** - Visual testing ground for all layout features
2. **Kitchen Sink** - Comprehensive interactive widget and hit testing example

Remove `run-wasm` and simplify example structure.

## Example 1: Layout Gallery

**Purpose**: Visual regression testing for layout engine features

### Window Configuration
- Title: "Layout Gallery - Sparsh"
- Size: 1000x800
- Background: Dark gray (#1F2937)

### Test Sections

**Row 1: Flex Direction**
- Column layout: 3 vertical boxes (red, green, blue)
- Row layout: 3 horizontal boxes (red, green, blue)
- Wrapped layout: 6 boxes that wrap when space runs out

**Row 2: Alignment**
- Align start: Items positioned at top
- Align center: Items centered vertically
- Align end: Items positioned at bottom

**Row 3: Spacing**
- No gap: Boxes touching each other
- Gap 8px: Small spacing between boxes
- Gap 24px: Large spacing between boxes

**Row 4: Sizing**
- Fixed size: 100x100 boxes
- Flex grow: Boxes expand to fill available space
- Min/max constraints: Boxes with size limits

**Row 5: Nesting**
- 3-level nested containers
- Alternating background colors for visibility
- Tests layout propagation through hierarchy

### Visual Design
- Each test case has a white text label explaining what it tests
- Vibrant, distinct colors for boxes (red, green, blue, orange, purple, pink)
- Light border around each test section (#374151)
- Padding: 16px between sections
- All content in a scrollable container

## Example 2: Kitchen Sink

**Purpose**: Interactive testing for widgets, events, and hit detection

### Window Configuration
- Title: "Kitchen Sink - Sparsh"
- Size: 1200x900
- Background: Dark theme (#0F172A)

### Layout Structure

**Left Sidebar (200px fixed width)**

*Button Gallery*
- Default button (blue #3B82F6)
- Success button (green #22C55E)
- Danger button (red #EF4444)
- Warning button (orange #F59E0B)
- Secondary button (gray #64748B)

*Text Display*
- Heading: "Typography" (size 24, bold)
- Body: "Body text example" (size 16)
- Caption: "Small caption text" (size 12, gray)

**Main Area (flex-grow, scrollable)**

*Section 1: Input Fields*
- Empty TextInput with placeholder "Enter text..."
- Pre-filled TextInput with "Hello World"
- Focused TextInput (different border color)
- Disabled TextInput (grayed out, if supported)
- Tests: Tab navigation, click focus, text entry

*Section 2: Container Tests*
- Nested containers (3 levels)
  - Outer: 400x300, padding 16px, blue tint
  - Middle: full width, padding 16px, green tint
  - Inner: full width, padding 16px, purple tint
- Overlapping containers (tests z-order)
  - Two 200x200 containers, offset by 50px
  - Different background colors
- Corner radius variations (0px, 8px, 16px, 32px)
- Semi-transparent containers (alpha 0.5, 0.7, 0.9)

*Section 3: Scroll Container*
- Scrollable area: 600x300 viewport, 600x800 content
- Contains 20 items (each 40px height)
- Alternating background colors for visibility
- Tests: Scroll wheel, scroll bar interaction, scroll hit detection

**Bottom Status Bar (fixed 40px height)**
- Current hover: "Hover: Button#42" or "Hover: None"
- Current focus: "Focus: TextInput#7" or "Focus: None"
- Last click: "Click: (342, 156)"
- FPS counter: "FPS: 60" (if available)

### Interaction Testing Focus
- Hover states on all buttons
- Click feedback (visual state change)
- Focus indicators on text inputs
- Scroll area interaction
- Hit testing in nested/overlapping containers
- Tab navigation between focusable elements

## Implementation Notes

### File Structure
```
examples/
├── layout/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── kitchen-sink/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── README.md (updated)
```

### Remove
- `run-wasm/` directory
- Web-specific code from examples
- `wasm-bindgen` dependencies from example Cargo.toml files

### Keep (for now)
- `triangle` example (minimal rendering test)
- `counter` example (stateful widget reference)
- `demo` example (can be updated/replaced later)
- `native-demo` example (native macOS widgets)

### Update
- Root `Cargo.toml`: Remove `run-wasm` from workspace members, add `layout` and `kitchen-sink`
- `examples/README.md`: Document the new examples

## Testing Checklist

### Layout Gallery
- [ ] All 5 rows render correctly
- [ ] Flex direction tests show correct layout
- [ ] Alignment tests position items correctly
- [ ] Spacing tests show accurate gaps
- [ ] Sizing tests respect constraints
- [ ] Nested containers render without visual glitches
- [ ] Scrolling works smoothly

### Kitchen Sink
- [ ] All buttons show hover states
- [ ] Buttons respond to clicks
- [ ] Text inputs accept keyboard input
- [ ] Tab key navigates between inputs
- [ ] Focus indicators are visible
- [ ] Nested containers render correctly
- [ ] Overlapping containers respect z-order
- [ ] Scroll container scrolls smoothly
- [ ] Status bar updates correctly
- [ ] No hit testing issues in nested/overlapping areas

## Success Criteria

1. Layout Gallery renders all test cases clearly
2. Kitchen Sink demonstrates all interactive widgets
3. Both examples build without errors
4. Both examples run without crashes
5. Hit testing works correctly in all scenarios
6. Examples serve as good reference for developers
7. Easy to add new test cases

## Future Enhancements (Post-Alpha)

- Performance metrics display
- Interactive layout parameter tweaking
- Visual diff testing support
- Screenshot/snapshot testing
- Web/WASM support (add back when ready)
