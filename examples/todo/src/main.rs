//! Todo example app for Spark (native + web).

use spark::prelude::*;
use spark::widgets::{EventContext, LayoutContext, PaintContext, WidgetId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

fn main() {
    #[cfg(target_arch = "wasm32")]
    spark::init_web();

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::new()
        .with_title("Spark Todo")
        .with_size(960, 720)
        .with_background(Color::from_hex(0x0F172A))
        .run(|| Box::new(TodoApp::new()));
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Active,
    Done,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TodoItem {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TodoAction {
    SetDraft(String),
    AddDraft,
    Toggle(u64),
    Delete(u64),
    SetFilter(Filter),
    ClearCompleted,
}

#[derive(Debug, Default)]
struct TodoModel {
    todos: Vec<TodoItem>,
    filter: Filter,
    draft: String,
    next_id: u64,
}

impl TodoModel {
    fn apply(&mut self, action: TodoAction) -> bool {
        match action {
            TodoAction::SetDraft(text) => {
                if self.draft != text {
                    self.draft = text;
                    true
                } else {
                    false
                }
            }
            TodoAction::AddDraft => {
                let text = self.draft.trim();
                if text.is_empty() {
                    return false;
                }
                let id = self.next_id;
                self.next_id += 1;
                self.todos.push(TodoItem {
                    id,
                    text: text.to_owned(),
                    done: false,
                });
                self.draft.clear();
                true
            }
            TodoAction::Toggle(id) => {
                if let Some(todo) = self.todos.iter_mut().find(|todo| todo.id == id) {
                    todo.done = !todo.done;
                    true
                } else {
                    false
                }
            }
            TodoAction::Delete(id) => {
                let before = self.todos.len();
                self.todos.retain(|todo| todo.id != id);
                before != self.todos.len()
            }
            TodoAction::SetFilter(filter) => {
                if self.filter != filter {
                    self.filter = filter;
                    true
                } else {
                    false
                }
            }
            TodoAction::ClearCompleted => {
                let before = self.todos.len();
                self.todos.retain(|todo| !todo.done);
                before != self.todos.len()
            }
        }
    }

    fn filtered_todos(&self) -> impl Iterator<Item = &TodoItem> {
        self.todos.iter().filter(move |todo| match self.filter {
            Filter::All => true,
            Filter::Active => !todo.done,
            Filter::Done => todo.done,
        })
    }
}

struct TodoApp {
    id: WidgetId,
    model: TodoModel,
    actions: Arc<Mutex<Vec<TodoAction>>>,
    children: Vec<Box<dyn Widget>>,
}

impl TodoApp {
    fn new() -> Self {
        let mut app = Self {
            id: WidgetId::default(),
            model: TodoModel::default(),
            actions: Arc::new(Mutex::new(Vec::new())),
            children: Vec::new(),
        };
        app.rebuild_children();
        app
    }

    #[cfg(test)]
    fn visible_count(&self) -> usize {
        self.model.filtered_todos().count()
    }

    fn process_action_queue(&mut self) -> bool {
        let actions: Vec<_> = {
            let mut queue = self
                .actions
                .lock()
                .expect("todo action queue mutex should not be poisoned");
            std::mem::take(&mut *queue)
        };
        if actions.is_empty() {
            return false;
        }

        let mut changed = false;
        for action in actions {
            changed |= self.model.apply(action);
        }

        if changed {
            self.rebuild_children();
        }

        changed
    }

    fn rebuild_children(&mut self) {
        let active_count = self.model.todos.iter().filter(|todo| !todo.done).count();
        let done_count = self.model.todos.iter().filter(|todo| todo.done).count();

        let mut list = List::new().vertical().gap(10.0).fill_width();
        let visible: Vec<TodoItem> = self.model.filtered_todos().cloned().collect();
        if visible.is_empty() {
            list.push_item(
                Container::new()
                    .padding(14.0)
                    .background(Color::from_hex(0x0B1220))
                    .corner_radius(8.0)
                    .child(
                        Text::new("No tasks for this filter yet.")
                            .size(14.0)
                            .color(Color::from_hex(0x94A3B8)),
                    ),
            );
        } else {
            for todo in visible {
                list.push_item(self.todo_row(todo));
            }
        }

        let content = Container::new()
            .column()
            .gap(14.0)
            .padding(26.0)
            .width(720.0)
            .background(Color::from_hex(0x111827))
            .corner_radius(14.0)
            .child(Text::new("Todo").size(28.0).bold().color(Color::WHITE))
            .child(
                Text::new("Spark native + web example using reusable Checkbox and List widgets.")
                    .size(13.0)
                    .color(Color::from_hex(0x9CA3AF)),
            )
            .child(self.input_row())
            .child(self.filter_row())
            .child(
                Scroll::new()
                    .vertical()
                    .fill_width()
                    .height(340.0)
                    .content(list),
            )
            .child(
                Container::new()
                    .row()
                    .fill_width()
                    .space_between()
                    .align_items(taffy::prelude::AlignItems::Center)
                    .child(
                        Text::new(format!(
                            "{} active / {} done / {} total",
                            active_count,
                            done_count,
                            self.model.todos.len()
                        ))
                        .size(13.0)
                        .color(Color::from_hex(0x9CA3AF)),
                    )
                    .child(ActionButton::from_button(
                        Button::new("Clear Completed")
                            .background(Color::from_hex(0x475569))
                            .text_color(Color::WHITE),
                        Arc::clone(&self.actions),
                        TodoAction::ClearCompleted,
                    )),
            );

        self.children = vec![Box::new(
            Container::new()
                .fill()
                .center()
                .background(Color::from_hex(0x0F172A))
                .child(content),
        )];
    }

    fn input_row(&self) -> Container {
        Container::new()
            .row()
            .fill_width()
            .gap(10.0)
            .align_items(taffy::prelude::AlignItems::Center)
            .child(ActionTextInput::new(
                self.model.draft.clone(),
                Arc::clone(&self.actions),
            ))
            .child(ActionButton::from_button(
                Button::new("Add")
                    .background(Color::from_hex(0x2563EB))
                    .text_color(Color::WHITE),
                Arc::clone(&self.actions),
                TodoAction::AddDraft,
            ))
    }

    fn filter_row(&self) -> Container {
        let queue = Arc::clone(&self.actions);
        Container::new()
            .row()
            .fill_width()
            .gap(8.0)
            .child(self.filter_button("All", Filter::All, Arc::clone(&queue)))
            .child(self.filter_button("Active", Filter::Active, Arc::clone(&queue)))
            .child(self.filter_button("Done", Filter::Done, queue))
    }

    fn filter_button(
        &self,
        label: &str,
        filter: Filter,
        queue: Arc<Mutex<Vec<TodoAction>>>,
    ) -> ActionButton {
        let selected = self.model.filter == filter;
        let background = if selected {
            Color::from_hex(0x2563EB)
        } else {
            Color::from_hex(0x334155)
        };
        ActionButton::from_button(
            Button::new(label)
                .background(background)
                .text_color(Color::WHITE),
            queue,
            TodoAction::SetFilter(filter),
        )
    }

    fn todo_row(&self, todo: TodoItem) -> Container {
        let queue = Arc::clone(&self.actions);
        let text_color = if todo.done {
            Color::from_hex(0x64748B)
        } else {
            Color::from_hex(0xE2E8F0)
        };
        let row_bg = if todo.done {
            Color::from_hex(0x0F172A)
        } else {
            Color::from_hex(0x1E293B)
        };

        Container::new()
            .row()
            .fill_width()
            .gap(12.0)
            .padding(12.0)
            .background(row_bg)
            .corner_radius(10.0)
            .align_items(taffy::prelude::AlignItems::Center)
            .child(ActionCheckbox::from_checkbox(
                Checkbox::with_checked(todo.done),
                Arc::clone(&queue),
                TodoAction::Toggle(todo.id),
            ))
            .child(
                Container::new()
                    .flex_grow(1.0)
                    .child(Text::new(todo.text).size(15.0).color(text_color)),
            )
            .child(ActionButton::from_button(
                Button::new("Delete")
                    .background(Color::from_hex(0x7F1D1D))
                    .text_color(Color::WHITE),
                queue,
                TodoAction::Delete(todo.id),
            ))
    }
}

impl Widget for TodoApp {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> taffy::Style {
        taffy::Style {
            size: taffy::prelude::Size {
                width: taffy::prelude::percent(1.0),
                height: taffy::prelude::percent(1.0),
            },
            ..Default::default()
        }
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) -> EventResponse {
        if self.process_action_queue() {
            EventResponse {
                repaint: true,
                relayout: true,
                ..Default::default()
            }
        } else {
            EventResponse::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}

struct ActionButton {
    inner: Button,
    fired: Arc<AtomicBool>,
}

impl ActionButton {
    fn from_button(button: Button, queue: Arc<Mutex<Vec<TodoAction>>>, action: TodoAction) -> Self {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_cb = Arc::clone(&fired);
        let queued_action = action.clone();
        let inner = button.on_click(move || {
            queue
                .lock()
                .expect("todo action queue mutex should not be poisoned")
                .push(queued_action.clone());
            fired_cb.store(true, Ordering::SeqCst);
        });
        Self { inner, fired }
    }
}

impl Widget for ActionButton {
    fn id(&self) -> WidgetId {
        self.inner.id()
    }

    fn set_id(&mut self, id: WidgetId) {
        self.inner.set_id(id);
    }

    fn style(&self) -> taffy::Style {
        self.inner.style()
    }

    fn paint(&self, ctx: &mut PaintContext) {
        self.inner.paint(ctx);
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) -> EventResponse {
        let mut response = self.inner.event(ctx, event);
        if self.fired.swap(false, Ordering::SeqCst) {
            response.handled = false;
            response.relayout = true;
            response.repaint = true;
        }
        response
    }

    fn focusable(&self) -> bool {
        self.inner.focusable()
    }

    fn measure(&self, ctx: &mut LayoutContext) -> Option<(f32, f32)> {
        self.inner.measure(ctx)
    }
}

struct ActionCheckbox {
    inner: Checkbox,
    fired: Arc<AtomicBool>,
}

impl ActionCheckbox {
    fn from_checkbox(
        checkbox: Checkbox,
        queue: Arc<Mutex<Vec<TodoAction>>>,
        action: TodoAction,
    ) -> Self {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_cb = Arc::clone(&fired);
        let queued_action = action.clone();
        let inner = checkbox.on_toggle(move |_| {
            queue
                .lock()
                .expect("todo action queue mutex should not be poisoned")
                .push(queued_action.clone());
            fired_cb.store(true, Ordering::SeqCst);
        });
        Self { inner, fired }
    }
}

impl Widget for ActionCheckbox {
    fn id(&self) -> WidgetId {
        self.inner.id()
    }

    fn set_id(&mut self, id: WidgetId) {
        self.inner.set_id(id);
    }

    fn style(&self) -> taffy::Style {
        self.inner.style()
    }

    fn paint(&self, ctx: &mut PaintContext) {
        self.inner.paint(ctx);
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) -> EventResponse {
        let mut response = self.inner.event(ctx, event);
        if self.fired.swap(false, Ordering::SeqCst) {
            response.handled = false;
            response.relayout = true;
            response.repaint = true;
        }
        response
    }

    fn focusable(&self) -> bool {
        self.inner.focusable()
    }

    fn measure(&self, ctx: &mut LayoutContext) -> Option<(f32, f32)> {
        self.inner.measure(ctx)
    }
}

struct ActionTextInput {
    inner: TextInput,
    fired: Arc<AtomicBool>,
}

impl ActionTextInput {
    fn new(draft: String, queue: Arc<Mutex<Vec<TodoAction>>>) -> Self {
        let fired = Arc::new(AtomicBool::new(false));
        let fired_change = Arc::clone(&fired);
        let fired_submit = Arc::clone(&fired);
        let queue_change = Arc::clone(&queue);
        let inner = TextInput::new()
            .value(draft)
            .fill_width()
            .placeholder("Add a task...")
            .on_change(move |value| {
                queue_change
                    .lock()
                    .expect("todo action queue mutex should not be poisoned")
                    .push(TodoAction::SetDraft(value.to_owned()));
                fired_change.store(true, Ordering::SeqCst);
            })
            .on_submit(move |_| {
                queue
                    .lock()
                    .expect("todo action queue mutex should not be poisoned")
                    .push(TodoAction::AddDraft);
                fired_submit.store(true, Ordering::SeqCst);
            });
        Self { inner, fired }
    }
}

impl Widget for ActionTextInput {
    fn id(&self) -> WidgetId {
        self.inner.id()
    }

    fn set_id(&mut self, id: WidgetId) {
        self.inner.set_id(id);
    }

    fn style(&self) -> taffy::Style {
        self.inner.style()
    }

    fn paint(&self, ctx: &mut PaintContext) {
        self.inner.paint(ctx);
    }

    fn event(&mut self, ctx: &mut EventContext, event: &InputEvent) -> EventResponse {
        let mut response = self.inner.event(ctx, event);
        if self.fired.swap(false, Ordering::SeqCst) {
            response.handled = false;
            response.relayout = true;
            response.repaint = true;
        }
        response
    }

    fn focusable(&self) -> bool {
        self.inner.focusable()
    }

    fn measure(&self, ctx: &mut LayoutContext) -> Option<(f32, f32)> {
        self.inner.measure(ctx)
    }

    fn on_focus(&mut self) {
        self.inner.on_focus();
    }

    fn on_blur(&mut self) {
        self.inner.on_blur();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_model_core_flow() {
        let mut model = TodoModel::default();
        assert!(model.apply(TodoAction::SetDraft("Write tests".to_owned())));
        assert!(model.apply(TodoAction::AddDraft));
        assert_eq!(model.todos.len(), 1);
        assert_eq!(model.draft, "");
        assert_eq!(model.todos[0].text, "Write tests");

        let id = model.todos[0].id;
        assert!(model.apply(TodoAction::Toggle(id)));
        assert!(model.todos[0].done);
        assert!(model.apply(TodoAction::SetFilter(Filter::Done)));
        assert_eq!(model.filtered_todos().count(), 1);
        assert!(model.apply(TodoAction::ClearCompleted));
        assert!(model.todos.is_empty());
    }

    #[test]
    fn todo_app_queue_processing_applies_actions_deterministically() {
        let mut app = TodoApp::new();
        app.actions
            .lock()
            .expect("todo action queue mutex should not be poisoned")
            .push(TodoAction::SetDraft("Alpha".to_owned()));
        app.actions
            .lock()
            .expect("todo action queue mutex should not be poisoned")
            .push(TodoAction::AddDraft);
        assert!(app.process_action_queue());
        assert_eq!(app.model.todos.len(), 1);
        assert_eq!(app.model.todos[0].text, "Alpha");

        let id = app.model.todos[0].id;
        app.actions
            .lock()
            .expect("todo action queue mutex should not be poisoned")
            .push(TodoAction::Toggle(id));
        app.actions
            .lock()
            .expect("todo action queue mutex should not be poisoned")
            .push(TodoAction::SetFilter(Filter::Done));
        assert!(app.process_action_queue());
        assert_eq!(app.visible_count(), 1);
        assert!(app.model.todos[0].done);

        app.actions
            .lock()
            .expect("todo action queue mutex should not be poisoned")
            .push(TodoAction::Delete(id));
        assert!(app.process_action_queue());
        assert_eq!(app.model.todos.len(), 0);
        assert_eq!(app.children.len(), 1);
    }
}
