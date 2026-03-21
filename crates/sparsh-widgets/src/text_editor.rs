//! Shared text editor state used by text input widgets.

use std::ops::Range;

/// Runtime-facing snapshot of a text editor widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditorState {
    pub text: String,
    pub cursor: usize,
    pub anchor: usize,
    pub multiline: bool,
    pub composing_range: Option<(usize, usize)>,
}

impl TextEditorState {
    pub fn selection_range(&self) -> (usize, usize) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }
}

#[derive(Clone, Debug)]
struct EditorSnapshot {
    text: String,
    cursor: usize,
    anchor: usize,
    preferred_column: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct EditorCore {
    text: String,
    cursor: usize,
    anchor: usize,
    preferred_column: Option<usize>,
    composing_range: Option<(usize, usize)>,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
}

impl EditorCore {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.len();
        Self {
            text,
            cursor,
            anchor: cursor,
            preferred_column: None,
            composing_range: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    #[cfg(test)]
    pub(crate) fn composing_range(&self) -> Option<(usize, usize)> {
        self.composing_range
    }

    pub(crate) fn state(&self, multiline: bool) -> TextEditorState {
        TextEditorState {
            text: self.text.clone(),
            cursor: self.cursor,
            anchor: self.anchor,
            multiline,
            composing_range: self.composing_range,
        }
    }

    pub(crate) fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.text.len();
        self.anchor = self.cursor;
        self.preferred_column = None;
        self.composing_range = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub(crate) fn selection_range(&self) -> Option<(usize, usize)> {
        (self.anchor != self.cursor).then_some({
            if self.anchor < self.cursor {
                (self.anchor, self.cursor)
            } else {
                (self.cursor, self.anchor)
            }
        })
    }

    pub(crate) fn selected_text(&self) -> Option<&str> {
        let (start, end) = self.selection_range()?;
        self.text.get(start..end)
    }

    pub(crate) fn set_cursor(&mut self, index: usize, extend: bool) {
        let clamped = self.clamp_to_boundary(index);
        if extend {
            self.cursor = clamped;
        } else {
            self.cursor = clamped;
            self.anchor = clamped;
        }
        self.preferred_column = None;
    }

    pub(crate) fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.text.len();
        self.preferred_column = None;
    }

    pub(crate) fn clear_selection(&mut self) {
        self.anchor = self.cursor;
        self.preferred_column = None;
    }

    pub(crate) fn insert_text(&mut self, text: &str, multiline: bool) -> bool {
        let inserted = sanitize_input(text, multiline);
        if inserted.is_empty() {
            return false;
        }
        self.replace_current_selection(&inserted, true);
        true
    }

    pub(crate) fn paste_text(&mut self, text: &str, multiline: bool) -> bool {
        self.insert_text(text, multiline)
    }

    pub(crate) fn begin_composition(&mut self) {
        if self.composing_range.is_none() {
            self.push_history();
            self.redo_stack.clear();
            let (start, end) = self.selection_range().unwrap_or((self.cursor, self.cursor));
            if start != end {
                self.text.replace_range(start..end, "");
                self.cursor = start;
                self.anchor = start;
            }
            self.composing_range = Some((self.cursor, self.cursor));
        }
    }

    pub(crate) fn update_composition(&mut self, text: &str, multiline: bool) -> bool {
        let inserted = sanitize_input(text, multiline);
        if self.composing_range.is_none() {
            self.begin_composition();
        }
        let Some((start, end)) = self.composing_range else {
            return false;
        };
        self.text.replace_range(start..end, &inserted);
        let next = start + inserted.len();
        self.cursor = next;
        self.anchor = next;
        self.composing_range = Some((start, next));
        self.preferred_column = None;
        true
    }

    pub(crate) fn end_composition(&mut self, text: &str, multiline: bool) -> bool {
        if self.composing_range.is_none() {
            return self.insert_text(text, multiline);
        }

        let inserted = sanitize_input(text, multiline);
        let (start, end) = self.composing_range.expect("checked above");
        self.text.replace_range(start..end, &inserted);
        let next = start + inserted.len();
        self.cursor = next;
        self.anchor = next;
        self.composing_range = None;
        self.preferred_column = None;
        true
    }

    pub(crate) fn clear_composition(&mut self) {
        self.composing_range = None;
    }

    pub(crate) fn backspace(&mut self) -> bool {
        if self.delete_selection_internal(true) {
            return true;
        }
        if self.cursor == 0 {
            return false;
        }

        self.push_history();
        self.redo_stack.clear();
        let prev = prev_boundary(&self.text, self.cursor);
        self.text.replace_range(prev..self.cursor, "");
        self.cursor = prev;
        self.anchor = prev;
        self.preferred_column = None;
        self.composing_range = None;
        true
    }

    pub(crate) fn delete_forward(&mut self) -> bool {
        if self.delete_selection_internal(true) {
            return true;
        }
        if self.cursor >= self.text.len() {
            return false;
        }

        self.push_history();
        self.redo_stack.clear();
        let next = next_boundary(&self.text, self.cursor);
        self.text.replace_range(self.cursor..next, "");
        self.anchor = self.cursor;
        self.preferred_column = None;
        self.composing_range = None;
        true
    }

    pub(crate) fn copy_selection(&self) -> Option<String> {
        self.selected_text().map(ToOwned::to_owned)
    }

    pub(crate) fn cut_selection(&mut self) -> Option<String> {
        let selected = self.copy_selection()?;
        let _ = self.delete_selection_internal(true);
        Some(selected)
    }

    pub(crate) fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.snapshot());
        self.restore(snapshot);
        true
    }

    pub(crate) fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.snapshot());
        self.restore(snapshot);
        true
    }

    pub(crate) fn move_left(&mut self, extend: bool) {
        let target = if let Some((start, end)) = self.selection_range() {
            if extend {
                prev_boundary(&self.text, self.cursor)
            } else if self.cursor > start {
                start
            } else {
                end
            }
        } else {
            prev_boundary(&self.text, self.cursor)
        };
        self.set_cursor_with_extend(target, extend);
    }

    pub(crate) fn move_right(&mut self, extend: bool) {
        let target = if let Some((start, end)) = self.selection_range() {
            if extend {
                next_boundary(&self.text, self.cursor)
            } else if self.cursor < end {
                end
            } else {
                start
            }
        } else {
            next_boundary(&self.text, self.cursor)
        };
        self.set_cursor_with_extend(target, extend);
    }

    pub(crate) fn move_word_left(&mut self, extend: bool) {
        self.set_cursor_with_extend(prev_word_boundary(&self.text, self.cursor), extend);
    }

    pub(crate) fn move_word_right(&mut self, extend: bool) {
        self.set_cursor_with_extend(next_word_boundary(&self.text, self.cursor), extend);
    }

    pub(crate) fn move_to_start(&mut self, extend: bool) {
        self.set_cursor_with_extend(0, extend);
    }

    pub(crate) fn move_to_end(&mut self, extend: bool) {
        self.set_cursor_with_extend(self.text.len(), extend);
    }

    pub(crate) fn move_up(&mut self, extend: bool) {
        let Some(target) = self.vertical_target(-1) else {
            if !extend {
                self.anchor = self.cursor;
            }
            return;
        };
        self.set_cursor_with_extend(target, extend);
    }

    pub(crate) fn move_down(&mut self, extend: bool) {
        let Some(target) = self.vertical_target(1) else {
            if !extend {
                self.anchor = self.cursor;
            }
            return;
        };
        self.set_cursor_with_extend(target, extend);
    }

    pub(crate) fn line_starts(&self) -> Vec<usize> {
        line_starts(&self.text)
    }

    pub(crate) fn line_and_column(&self, index: usize) -> (usize, usize) {
        line_and_column(&self.text, index)
    }

    pub(crate) fn line_count(&self) -> usize {
        self.line_starts().len()
    }

    fn vertical_target(&mut self, delta: isize) -> Option<usize> {
        let (line, column) = line_and_column(&self.text, self.cursor);
        let desired_column = *self.preferred_column.get_or_insert(column);
        let line_count = self.line_count();
        let target_line = if delta.is_negative() {
            line.checked_sub(delta.unsigned_abs())?
        } else {
            let next = line + delta as usize;
            if next >= line_count {
                return None;
            }
            next
        };
        Some(index_for_line_column(
            &self.text,
            target_line,
            desired_column,
        ))
    }

    fn set_cursor_with_extend(&mut self, target: usize, extend: bool) {
        let target = self.clamp_to_boundary(target);
        if extend {
            self.cursor = target;
        } else {
            self.cursor = target;
            self.anchor = target;
        }
        self.composing_range = None;
    }

    fn replace_current_selection(&mut self, replacement: &str, track_history: bool) {
        if track_history {
            self.push_history();
            self.redo_stack.clear();
        }
        let (start, end) = self.selection_range().unwrap_or((self.cursor, self.cursor));
        self.text.replace_range(start..end, replacement);
        let next = start + replacement.len();
        self.cursor = next;
        self.anchor = next;
        self.preferred_column = None;
        self.composing_range = None;
    }

    fn delete_selection_internal(&mut self, track_history: bool) -> bool {
        let Some((start, end)) = self.selection_range() else {
            return false;
        };
        if track_history {
            self.push_history();
            self.redo_stack.clear();
        }
        self.text.replace_range(start..end, "");
        self.cursor = start;
        self.anchor = start;
        self.preferred_column = None;
        self.composing_range = None;
        true
    }

    fn clamp_to_boundary(&self, index: usize) -> usize {
        if index >= self.text.len() {
            return self.text.len();
        }
        self.text
            .char_indices()
            .map(|(idx, _)| idx)
            .find(|idx| *idx >= index)
            .unwrap_or(self.text.len())
    }

    fn push_history(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > 128 {
            let _ = self.undo_stack.remove(0);
        }
    }

    fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
            anchor: self.anchor,
            preferred_column: self.preferred_column,
        }
    }

    fn restore(&mut self, snapshot: EditorSnapshot) {
        self.text = snapshot.text;
        self.cursor = snapshot.cursor;
        self.anchor = snapshot.anchor;
        self.preferred_column = snapshot.preferred_column;
        self.composing_range = None;
    }
}

fn sanitize_input(text: &str, multiline: bool) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    normalized
        .chars()
        .filter(|ch| {
            if multiline && *ch == '\n' {
                return true;
            }
            !ch.is_control()
        })
        .collect()
}

fn prev_boundary(text: &str, index: usize) -> usize {
    if index == 0 {
        return 0;
    }
    text[..index]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    text[index..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| index + offset)
        .unwrap_or(text.len())
}

fn prev_word_boundary(text: &str, index: usize) -> usize {
    let mut cursor = index;
    while cursor > 0 {
        let prev = prev_boundary(text, cursor);
        let ch = text[prev..cursor].chars().next().unwrap_or(' ');
        cursor = prev;
        if !ch.is_whitespace() {
            break;
        }
    }
    while cursor > 0 {
        let prev = prev_boundary(text, cursor);
        let ch = text[prev..cursor].chars().next().unwrap_or(' ');
        if ch.is_whitespace() {
            break;
        }
        cursor = prev;
    }
    cursor
}

fn next_word_boundary(text: &str, index: usize) -> usize {
    let mut cursor = index;
    while cursor < text.len() {
        let next = next_boundary(text, cursor);
        let ch = text[cursor..next].chars().next().unwrap_or(' ');
        if !ch.is_whitespace() {
            break;
        }
        cursor = next;
    }
    while cursor < text.len() {
        let next = next_boundary(text, cursor);
        let ch = text[cursor..next].chars().next().unwrap_or(' ');
        cursor = next;
        if ch.is_whitespace() {
            break;
        }
    }
    cursor
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in text.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_range(text: &str, line: usize) -> Range<usize> {
    let starts = line_starts(text);
    let start = starts.get(line).copied().unwrap_or(text.len());
    let end = starts
        .get(line + 1)
        .copied()
        .map(|next| next.saturating_sub(1))
        .unwrap_or(text.len());
    start..end
}

fn line_and_column(text: &str, index: usize) -> (usize, usize) {
    let clamped = index.min(text.len());
    let starts = line_starts(text);
    let mut line = 0usize;
    for (idx, start) in starts.iter().enumerate() {
        if *start > clamped {
            break;
        }
        line = idx;
    }
    let start = starts.get(line).copied().unwrap_or(0);
    let column = text[start..clamped].chars().count();
    (line, column)
}

fn index_for_line_column(text: &str, line: usize, column: usize) -> usize {
    let range = line_range(text, line);
    let slice = &text[range.clone()];
    for (seen, (offset, _)) in slice.char_indices().enumerate() {
        if seen == column {
            return range.start + offset;
        }
    }
    range.end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_movement_uses_whitespace_boundaries() {
        let mut editor = EditorCore::new("alpha beta gamma");
        editor.move_word_left(false);
        assert_eq!(editor.cursor(), 11);
        editor.move_word_left(false);
        assert_eq!(editor.cursor(), 6);
        editor.move_word_right(false);
        assert_eq!(editor.cursor(), 11);
    }

    #[test]
    fn multiline_vertical_movement_preserves_column() {
        let mut editor = EditorCore::new("ab\ncdef\nxy");
        editor.set_cursor(4, false);
        editor.move_down(false);
        assert_eq!(editor.cursor(), 9);
        editor.move_up(false);
        assert_eq!(editor.cursor(), 4);
    }

    #[test]
    fn composition_replaces_current_selection() {
        let mut editor = EditorCore::new("hello");
        editor.set_cursor(5, false);
        editor.begin_composition();
        editor.update_composition("ka", false);
        assert_eq!(editor.text(), "helloka");
        editor.end_composition("kan", false);
        assert_eq!(editor.text(), "hellokan");
        assert!(editor.composing_range().is_none());
    }
}
