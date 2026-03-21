//! Paint tests for draw command emission.

use sparsh_core::Rect;
use sparsh_input::FocusManager;
use sparsh_layout::{ComputedLayout, LayoutTree};
use sparsh_render::{DrawCommand, DrawList};
use sparsh_text::TextSystem;
use sparsh_widgets::{Button, PaintCommands, PaintContext, Widget};

#[test]
fn button_paint_emits_rect_and_text_commands() {
    let mut draw_list = DrawList::new();
    let layout = ComputedLayout::new(Rect::new(0.0, 0.0, 100.0, 40.0));
    let layout_tree = LayoutTree::new();
    let focus = FocusManager::new();
    let mut text_system = TextSystem::new_headless();
    let mut paint_commands = PaintCommands::default();

    let button = Button::new("OK");

    let mut ctx = PaintContext {
        draw_list: &mut draw_list,
        layout,
        layout_tree: &layout_tree,
        focus: &focus,
        widget_id: button.id(),
        scale_factor: 1.0,
        text_system: &mut text_system,
        elapsed_time: 0.0,
        commands: &mut paint_commands,
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
        .any(|c| matches!(c, DrawCommand::TextRun { .. }));
    assert!(has_text, "button should emit a TextRun command for label");
}

#[test]
fn button_text_run_contains_label() {
    let mut draw_list = DrawList::new();
    let layout = ComputedLayout::new(Rect::new(0.0, 0.0, 120.0, 40.0));
    let layout_tree = LayoutTree::new();
    let focus = FocusManager::new();
    let mut text_system = TextSystem::new_headless();
    let mut paint_commands = PaintCommands::default();

    let button = Button::new("Hello");
    let mut ctx = PaintContext {
        draw_list: &mut draw_list,
        layout,
        layout_tree: &layout_tree,
        focus: &focus,
        widget_id: button.id(),
        scale_factor: 1.0,
        text_system: &mut text_system,
        elapsed_time: 0.0,
        commands: &mut paint_commands,
    };

    button.paint(&mut ctx);
    let run = draw_list.commands().iter().find_map(|command| {
        if let DrawCommand::TextRun { run } = command {
            Some(run)
        } else {
            None
        }
    });
    assert!(run.is_some(), "expected TextRun command");
    assert_eq!(run.expect("checked above").text, "Hello");
}
