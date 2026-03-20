//! Paint tests using headless wgpu. Require a GPU; run with `cargo test --ignored` or
//! `cargo test -p spark-widgets paint_ -- --ignored`.

#![cfg(not(target_arch = "wasm32"))]

use spark_core::{init_wgpu_headless, Rect};
use spark_input::FocusManager;
use spark_layout::{ComputedLayout, LayoutTree};
use spark_render::{DrawCommand, DrawList};
use spark_text::TextSystem;
use spark_widgets::{Button, PaintContext, Widget};

#[test]
#[ignore = "requires GPU; run with cargo test --ignored"]
fn button_paint_emits_rect_and_text_commands() {
    let (device, queue) = pollster::block_on(init_wgpu_headless()).expect("init headless wgpu");

    let mut draw_list = DrawList::new();
    let layout = ComputedLayout::new(Rect::new(0.0, 0.0, 100.0, 40.0));
    let layout_tree = LayoutTree::new();
    let focus = FocusManager::new();
    let mut text_system = TextSystem::new(&device);

    let button = Button::new("OK");

    let mut ctx = PaintContext {
        draw_list: &mut draw_list,
        layout,
        layout_tree: &layout_tree,
        focus: &focus,
        widget_id: button.id(),
        scale_factor: 1.0,
        text_system: &mut text_system,
        device: &device,
        queue: &queue,
        elapsed_time: 0.0,
    };

    button.paint(&mut ctx);

    let commands = draw_list.commands();
    assert!(
        !commands.is_empty(),
        "button should emit at least one draw command"
    );

    let has_rect = commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Rect { .. }));
    assert!(has_rect, "button should emit a Rect command for background");

    let has_text = commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { .. }));
    assert!(has_text, "button should emit a Text command for label");
}
