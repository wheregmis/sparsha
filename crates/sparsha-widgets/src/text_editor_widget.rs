//! Shared widget-layer helpers for text editor components.

use crate::text_editor::EditorCore;
use crate::text_input::TextInputStyle;
use crate::{
    control_state::{focus_ring_border_width, focus_ring_bounds, focus_ring_color},
    current_theme, responsive_text_area_min_height, responsive_theme_controls,
    responsive_typography, BuildContext, EventContext, PaintContext,
};
use sparsha_core::{Color, Rect as PaintRect};
use sparsha_input::StandardAction;
use sparsha_text::TextStyle;
use taffy::prelude::*;

#[derive(Clone)]
pub(crate) struct EditorBuildState {
    pub editor: EditorCore,
    pub declared_value: Option<String>,
    pub multiline: bool,
}

pub(crate) fn restore_editor_build_state(
    editor: &mut EditorCore,
    declared_value: &Option<String>,
    multiline: bool,
    ctx: &mut BuildContext,
) {
    if let Some(state) = ctx
        .take_boxed_state()
        .and_then(|state| state.downcast::<EditorBuildState>().ok())
        .map(|state| *state)
    {
        if state.multiline == multiline && state.declared_value == *declared_value {
            *editor = state.editor;
        }
    }
}

pub(crate) fn persist_editor_build_state(
    editor: &EditorCore,
    declared_value: &Option<String>,
    multiline: bool,
    ctx: &mut BuildContext,
) {
    ctx.store_boxed_state(Box::new(EditorBuildState {
        editor: editor.clone(),
        declared_value: declared_value.clone(),
        multiline,
    }));
}

pub(crate) fn compute_prefix_widths(
    text: &str,
    start: usize,
    mut measure_width: impl FnMut(&str) -> f32,
) -> Vec<(usize, f32)> {
    let mut boundaries = vec![(start, 0.0)];
    if text.is_empty() {
        return boundaries;
    }

    for (offset, _) in text.char_indices().skip(1) {
        boundaries.push((start + offset, measure_width(&text[..offset])));
    }
    boundaries.push((start + text.len(), measure_width(text)));
    boundaries
}

pub(crate) fn themed_editor_style(multiline: bool) -> TextInputStyle {
    let theme = current_theme();
    let controls = responsive_theme_controls(&theme);
    let typography = responsive_typography(&theme);

    TextInputStyle {
        background: theme.colors.input_background,
        background_focused: theme.colors.surface,
        text_color: theme.colors.text_primary,
        placeholder_color: theme.colors.input_placeholder,
        border_color: theme.colors.border,
        border_color_focused: theme.colors.primary,
        border_width: 1.0,
        corner_radius: theme.radii.md,
        padding_h: controls.control_padding_x,
        padding_v: controls.control_padding_y,
        font_size: typography.body_size,
        min_width: 180.0,
        min_height: if multiline {
            responsive_text_area_min_height(&theme)
        } else {
            controls.control_height
        },
    }
}

pub(crate) fn resolve_editor_style(
    style: &TextInputStyle,
    use_theme_defaults: bool,
    multiline: bool,
) -> TextInputStyle {
    if use_theme_defaults {
        themed_editor_style(multiline)
    } else {
        style.clone()
    }
}

pub(crate) fn editor_widget_style(style: &TextInputStyle, fill_width: bool) -> Style {
    Style {
        size: Size {
            width: if fill_width { percent(1.0) } else { auto() },
            height: auto(),
        },
        padding: Rect {
            left: length(style.padding_h),
            right: length(style.padding_h),
            top: length(style.padding_v),
            bottom: length(style.padding_v),
        },
        min_size: Size {
            width: length(style.min_width),
            height: length(style.min_height),
        },
        ..Default::default()
    }
}

pub(crate) fn editor_text_style(style: &TextInputStyle) -> TextStyle {
    TextStyle::default()
        .with_family(current_theme().typography.font_family.clone())
        .with_size(style.font_size)
        .with_color(style.text_color)
}

pub(crate) fn editor_placeholder_style(style: &TextInputStyle) -> TextStyle {
    TextStyle::default()
        .with_family(current_theme().typography.font_family.clone())
        .with_size(style.font_size)
        .with_color(style.placeholder_color)
}

pub(crate) fn paint_editor_frame(
    ctx: &mut PaintContext,
    bounds: PaintRect,
    style: &TextInputStyle,
) {
    let focused = ctx.has_focus();
    let scale = ctx.scale_factor;

    let background = if focused {
        style.background_focused
    } else {
        style.background
    };
    let border = if focused {
        style.border_color_focused
    } else {
        style.border_color
    };

    ctx.fill_bordered_rect(
        bounds,
        background,
        style.corner_radius,
        style.border_width,
        border,
    );

    if focused {
        let controls = current_theme().controls;
        let focus_bounds = focus_ring_bounds(bounds, scale, &controls);
        ctx.fill_bordered_rect(
            focus_bounds,
            Color::TRANSPARENT,
            style.corner_radius + 2.0,
            focus_ring_border_width(scale, &controls),
            focus_ring_color(current_theme().colors.border_focus),
        );
    }
}

pub(crate) fn apply_standard_action(
    editor: &mut EditorCore,
    ctx: &mut EventContext,
    action: StandardAction,
    multiline: bool,
) -> bool {
    let changed = match action {
        StandardAction::SelectAll => {
            editor.select_all();
            ctx.request_paint();
            false
        }
        StandardAction::Copy => {
            if let Some(text) = editor.copy_selection() {
                ctx.write_clipboard(text);
            }
            ctx.request_paint();
            false
        }
        StandardAction::Cut => {
            if let Some(text) = editor.cut_selection() {
                ctx.write_clipboard(text);
                ctx.request_layout();
                true
            } else {
                false
            }
        }
        StandardAction::Undo => {
            let changed = editor.undo();
            if changed {
                ctx.request_layout();
            }
            changed
        }
        StandardAction::Redo => {
            let changed = editor.redo();
            if changed {
                ctx.request_layout();
            }
            changed
        }
        StandardAction::Backspace => {
            let changed = editor.backspace();
            if changed {
                ctx.request_layout();
            }
            changed
        }
        StandardAction::Delete => {
            let changed = editor.delete_forward();
            if changed {
                ctx.request_layout();
            }
            changed
        }
        StandardAction::MoveLeft => {
            editor.move_left(false);
            ctx.request_paint();
            false
        }
        StandardAction::MoveRight => {
            editor.move_right(false);
            ctx.request_paint();
            false
        }
        StandardAction::SelectLeft => {
            editor.move_left(true);
            ctx.request_paint();
            false
        }
        StandardAction::SelectRight => {
            editor.move_right(true);
            ctx.request_paint();
            false
        }
        StandardAction::MoveWordLeft => {
            editor.move_word_left(false);
            ctx.request_paint();
            false
        }
        StandardAction::MoveWordRight => {
            editor.move_word_right(false);
            ctx.request_paint();
            false
        }
        StandardAction::SelectWordLeft => {
            editor.move_word_left(true);
            ctx.request_paint();
            false
        }
        StandardAction::SelectWordRight => {
            editor.move_word_right(true);
            ctx.request_paint();
            false
        }
        StandardAction::MoveToStart => {
            editor.move_to_start(false);
            ctx.request_paint();
            false
        }
        StandardAction::MoveToEnd => {
            editor.move_to_end(false);
            ctx.request_paint();
            false
        }
        StandardAction::SelectToStart => {
            editor.move_to_start(true);
            ctx.request_paint();
            false
        }
        StandardAction::SelectToEnd => {
            editor.move_to_end(true);
            ctx.request_paint();
            false
        }
        StandardAction::MoveUp if multiline => {
            editor.move_up(false);
            ctx.request_paint();
            false
        }
        StandardAction::MoveDown if multiline => {
            editor.move_down(false);
            ctx.request_paint();
            false
        }
        StandardAction::SelectUp if multiline => {
            editor.move_up(true);
            ctx.request_paint();
            false
        }
        StandardAction::SelectDown if multiline => {
            editor.move_down(true);
            ctx.request_paint();
            false
        }
        _ => false,
    };

    ctx.stop_propagation();
    changed
}

pub(crate) fn set_editor_value(editor: &mut EditorCore, value: Option<String>) -> bool {
    let next = value.unwrap_or_default();
    if editor.text() == next {
        return false;
    }
    editor.set_text(next);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::BuildStateStore;
    use std::any::Any;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MockStateStore {
        states: HashMap<Vec<usize>, Box<dyn Any>>,
    }

    impl BuildStateStore for MockStateStore {
        fn mark_path_used(&mut self, _path: &[usize]) {}

        fn take_boxed_state(&mut self, path: &[usize]) -> Option<Box<dyn Any>> {
            self.states.remove(path)
        }

        fn store_boxed_state(&mut self, path: Vec<usize>, state: Box<dyn Any>) {
            self.states.insert(path, state);
        }
    }

    #[test]
    fn matching_editor_build_state_restores_editor() {
        let mut store = MockStateStore::default();
        let mut build = BuildContext::default();
        // SAFETY: the test owns the store for the full build context use.
        unsafe { build.set_state_store(&mut store) };

        let mut original = EditorCore::new("hello");
        original.move_left(false);
        persist_editor_build_state(&original, &None, false, &mut build);

        let mut restored = EditorCore::new("");
        restore_editor_build_state(&mut restored, &None, false, &mut build);

        assert_eq!(restored.text(), "hello");
        assert_eq!(restored.cursor(), original.cursor());
    }

    #[test]
    fn editor_build_state_does_not_restore_across_single_and_multiline_modes() {
        let mut store = MockStateStore::default();
        let mut build = BuildContext::default();
        // SAFETY: the test owns the store for the full build context use.
        unsafe { build.set_state_store(&mut store) };

        let original = EditorCore::new("hello");
        persist_editor_build_state(&original, &None, false, &mut build);

        let mut restored = EditorCore::new("");
        restore_editor_build_state(&mut restored, &None, true, &mut build);

        assert_eq!(restored.text(), "");
    }
}
