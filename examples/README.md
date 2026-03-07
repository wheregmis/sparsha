# Spark Examples

This directory contains example applications demonstrating various features and capabilities of the Spark UI framework.

## Quick Start

All examples can be run using:

```bash
cargo run -p <example-name> --release
```

For development (faster compilation, slower runtime):

```bash
cargo run -p <example-name>
```

## Examples Overview

### 1. Triangle
**Path:** `examples/triangle`
**Level:** Beginner
**Topics:** Minimal setup, Typed pipeline, Basic rendering

A minimal example showing how to render a simple animated triangle using Spark's typed pipeline architecture. This is the best starting point for understanding the basics of Spark.

**What you'll learn:**
- Basic Spark application structure
- Typed pipeline setup
- Simple rendering with vertex buffers
- Basic animation using time uniforms

**Run:**
```bash
cargo run -p triangle --release
```

---

### 2. Counter
**Path:** `examples/counter`
**Level:** Beginner
**Topics:** State management, Basic widgets, Event handling

A simple counter application demonstrating basic state management and UI interactions in Spark.

**What you'll learn:**
- Application state management
- Button widgets and event handling
- Text display and formatting
- Basic layout composition

**Run:**
```bash
cargo run -p counter --release
```

---

### 3. Layout Gallery
**Path:** `examples/layout`
**Level:** Intermediate
**Topics:** Layout system, Flexbox, Alignment, Spacing

A comprehensive showcase of Spark's layout capabilities, including flexbox, alignment, padding, margins, and responsive design patterns.

**What you'll learn:**
- Flexbox layout (row, column, wrap)
- Alignment options (start, center, end, stretch, space-between)
- Padding and margin controls
- Nested layouts and composition
- Responsive design patterns
- Gap/spacing controls

**Run:**
```bash
cargo run -p layout --release
```

**Features:**
- Multiple layout patterns demonstrated
- Interactive examples with labels
- Clear visual hierarchy
- Practical layout recipes

---

### 4. Demo
**Path:** `examples/demo`
**Level:** Intermediate
**Topics:** Widgets library, Styling, Complex UI

A comprehensive demonstration of Spark's widget library and styling capabilities.

**What you'll learn:**
- All available widgets (buttons, text inputs, sliders, etc.)
- Widget styling and theming
- Form handling
- Complex UI composition

**Run:**
```bash
cargo run -p demo --release
```

---

### 5. Kitchen Sink
**Path:** `examples/kitchen-sink`
**Level:** Advanced
**Topics:** All features, Complex patterns, Best practices

A comprehensive example showcasing all major Spark features in a single application. This is a reference implementation demonstrating production-ready patterns.

**What you'll learn:**
- Complex application architecture
- Multiple views and navigation
- Advanced state management
- Animation and transitions
- Performance optimization
- Accessibility features
- Theming and styling
- Event handling patterns
- Real-world UI patterns

**Run:**
```bash
cargo run -p kitchen-sink --release
```

**Features:**
- Tab-based navigation
- Multiple example sections:
  - Widget showcase
  - Layout patterns
  - Animation gallery
  - Form handling
  - Data visualization
- Production-ready code patterns
- Accessibility best practices
- Performance optimizations

---

## Learning Path

We recommend exploring the examples in this order:

1. **Triangle** - Understand basic rendering
2. **Counter** - Learn state management and widgets
3. **Layout Gallery** - Master the layout system
4. **Demo** - Explore the widget library
5. **Kitchen Sink** - Study complex patterns and best practices

## Building All Examples

To verify all examples build correctly:

```bash
cargo build --examples --release
```

To check the entire workspace:

```bash
cargo check --workspace
```

## Development Tips

### Performance

- Always use `--release` for better performance when testing UI
- Development builds are much slower but compile faster
- Use `cargo build --examples` to build all examples at once

### Debugging

- Set `RUST_LOG=debug` for detailed logging:
  ```bash
  RUST_LOG=debug cargo run -p <example-name>
  ```

- Use `RUST_BACKTRACE=1` for stack traces on panics:
  ```bash
  RUST_BACKTRACE=1 cargo run -p <example-name>
  ```

### Platform-Specific Notes

**macOS:**
- Metal backend is used for rendering

**Windows:**
- DirectX 12 backend is preferred
- Fallback to DirectX 11 if DX12 unavailable

**Linux:**
- Vulkan backend is used

## Contributing

When adding new examples:

1. Create a new directory in `examples/`
2. Add a `Cargo.toml` with appropriate dependencies
3. Add a `src/main.rs` with your example
4. Update this README with a description
5. Ensure it builds with `cargo build --examples`
6. Test on all supported platforms if possible

## Troubleshooting

### Build Errors

If you encounter build errors:

```bash
# Clean and rebuild
cargo clean
cargo build --examples

# Update dependencies
cargo update
```

### Runtime Issues

- Ensure your graphics drivers are up to date
- Check that your platform supports the required graphics APIs
- Ensure required system libraries are installed for your platform

### Platform-Specific Issues

**Linux:**
- Ensure Vulkan development libraries are installed for your distribution if needed.

**macOS:**
- Ensure Xcode Command Line Tools are installed:
  ```bash
  xcode-select --install
  ```

**Windows:**
- Ensure Visual Studio Build Tools are installed
- Windows SDK is required

## Additional Resources

- [Spark Documentation](../../docs/)
- [API Reference](../../docs/api/)
- [Contributing Guide](../../CONTRIBUTING.md)
- [Discord Community](https://discord.gg/spark) *(if available)*

## License

All examples are provided under the same license as the Spark project. See [LICENSE](../../LICENSE) for details.
