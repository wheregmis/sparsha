//! Todo example app for Sparsh (native + web) using signal-based state.

use serde_json::json;
use sparsh::prelude::*;
use sparsh::widgets::{BuildContext, EventContext, PaintContext, WidgetId};

fn main() {
    #[cfg(target_arch = "wasm32")]
    sparsh::init_web();

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    App::new()
        .with_title("Sparsh Todo")
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

#[derive(Clone, Debug, Default)]
struct TodoModel {
    todos: Vec<TodoItem>,
    filter: Filter,
    draft: String,
    next_id: u64,
}

impl TodoModel {
    fn apply(&mut self, action: TodoAction) {
        match action {
            TodoAction::SetDraft(text) => {
                self.draft = text;
            }
            TodoAction::AddDraft => {
                let text = self.draft.trim().to_owned();
                if text.is_empty() {
                    return;
                }
                let id = self.next_id;
                self.next_id += 1;
                self.todos.push(TodoItem {
                    id,
                    text,
                    done: false,
                });
                self.draft.clear();
            }
            TodoAction::Toggle(id) => {
                if let Some(todo) = self.todos.iter_mut().find(|todo| todo.id == id) {
                    todo.done = !todo.done;
                }
            }
            TodoAction::Delete(id) => {
                self.todos.retain(|todo| todo.id != id);
            }
            TodoAction::SetFilter(filter) => {
                self.filter = filter;
            }
            TodoAction::ClearCompleted => {
                self.todos.retain(|todo| !todo.done);
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

fn apply_action(model: Signal<TodoModel>, action: TodoAction) {
    model.with_mut(|m| {
        m.apply(action);
    });
}

struct TodoApp {
    id: WidgetId,
    model: Signal<TodoModel>,
    analysis_text: Signal<String>,
    analysis_generation: Signal<u64>,
    task_runtime: TaskRuntime,
    children: Vec<Box<dyn Widget>>,
}

fn value_to_u64(value: Option<&serde_json::Value>) -> u64 {
    match value {
        Some(v) => v
            .as_u64()
            .or_else(|| {
                v.as_f64().and_then(|num| {
                    if num.is_finite() && num >= 0.0 {
                        Some(num as u64)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0),
        None => 0,
    }
}

impl TodoApp {
    fn new() -> Self {
        let task_runtime = TaskRuntime::current_or_default();
        let analysis_text = Signal::new(String::from("Background analyzer is idle."));
        let analysis_for_results = analysis_text;
        task_runtime.on_result(move |result| {
            if result.task_kind != "analyze_text" {
                return;
            }
            match result.status {
                TaskStatus::Success(payload) => {
                    let words = value_to_u64(payload.get("word_count"));
                    let chars = value_to_u64(payload.get("char_count"));
                    let lines = value_to_u64(payload.get("line_count"));
                    analysis_for_results.set(format!(
                        "Background analyzer: {} words, {} chars, {} lines",
                        words, chars, lines
                    ));
                }
                TaskStatus::Error(message) => {
                    analysis_for_results.set(format!("Background analyzer error: {}", message));
                }
                TaskStatus::Canceled => {}
            }
        });

        let mut app = Self {
            id: WidgetId::default(),
            model: Signal::new(TodoModel::default()),
            analysis_text,
            analysis_generation: Signal::new(0),
            task_runtime,
            children: Vec::new(),
        };
        app.rebuild_children();
        app
    }

    #[cfg(test)]
    fn snapshot_model(&self) -> TodoModel {
        self.model.get()
    }

    fn rebuild_children(&mut self) {
        let model = self.model.get();
        let analysis_text = self.analysis_text.get();
        let active_count = model.todos.iter().filter(|todo| !todo.done).count();
        let done_count = model.todos.iter().filter(|todo| todo.done).count();

        let mut list = List::new().vertical().gap(10.0).fill_width();
        let visible: Vec<TodoItem> = model.filtered_todos().cloned().collect();
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
                Text::new("Sparsh native + web example using signal-driven state.")
                    .size(13.0)
                    .color(Color::from_hex(0x9CA3AF)),
            )
            .child(
                Text::new(analysis_text)
                    .size(12.0)
                    .color(Color::from_hex(0x93C5FD)),
            )
            .child(self.input_row(&model))
            .child(self.filter_row(model.filter))
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
                            model.todos.len()
                        ))
                        .size(13.0)
                        .color(Color::from_hex(0x9CA3AF)),
                    )
                    .child(self.clear_completed_button()),
            );

        self.children = vec![Box::new(
            Container::new()
                .fill()
                .center()
                .background(Color::from_hex(0x0F172A))
                .child(content),
        )];
    }

    fn input_row(&self, model: &TodoModel) -> Container {
        let model_for_change = self.model;
        let model_for_submit = self.model;
        let model_for_add = self.model;
        let runtime_for_analyze = self.task_runtime.clone();
        let generation_signal = self.analysis_generation;

        Container::new()
            .row()
            .fill_width()
            .gap(10.0)
            .align_items(taffy::prelude::AlignItems::Center)
            .child(
                TextInput::new()
                    .value(model.draft.clone())
                    .fill_width()
                    .placeholder("Add a task...")
                    .on_change(move |value| {
                        apply_action(model_for_change, TodoAction::SetDraft(value.to_owned()));
                        let generation = generation_signal.with_mut(|counter| {
                            *counter += 1;
                            *counter
                        });
                        runtime_for_analyze.spawn_keyed(
                            "todo.input-analysis",
                            generation,
                            "analyze_text",
                            json!({ "text": value }),
                        );
                    })
                    .on_submit(move |_| {
                        apply_action(model_for_submit, TodoAction::AddDraft);
                    }),
            )
            .child(
                Button::new("Add")
                    .background(Color::from_hex(0x2563EB))
                    .text_color(Color::WHITE)
                    .on_click(move || {
                        apply_action(model_for_add, TodoAction::AddDraft);
                    }),
            )
    }

    fn filter_row(&self, current_filter: Filter) -> Container {
        Container::new()
            .row()
            .fill_width()
            .gap(8.0)
            .child(self.filter_button("All", Filter::All, current_filter))
            .child(self.filter_button("Active", Filter::Active, current_filter))
            .child(self.filter_button("Done", Filter::Done, current_filter))
    }

    fn filter_button(&self, label: &str, filter: Filter, current_filter: Filter) -> Button {
        let selected = current_filter == filter;
        let background = if selected {
            Color::from_hex(0x2563EB)
        } else {
            Color::from_hex(0x334155)
        };
        let model = self.model;
        Button::new(label)
            .background(background)
            .text_color(Color::WHITE)
            .on_click(move || {
                apply_action(model, TodoAction::SetFilter(filter));
            })
    }

    fn clear_completed_button(&self) -> Button {
        let model = self.model;
        Button::new("Clear Completed")
            .background(Color::from_hex(0x475569))
            .text_color(Color::WHITE)
            .on_click(move || {
                apply_action(model, TodoAction::ClearCompleted);
            })
    }

    fn todo_row(&self, todo: TodoItem) -> Container {
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

        let id = todo.id;
        let model_for_toggle = self.model;
        let model_for_delete = self.model;

        Container::new()
            .row()
            .fill_width()
            .gap(12.0)
            .padding(12.0)
            .background(row_bg)
            .corner_radius(10.0)
            .align_items(taffy::prelude::AlignItems::Center)
            .child(Checkbox::with_checked(todo.done).on_toggle(move |_| {
                apply_action(model_for_toggle, TodoAction::Toggle(id));
            }))
            .child(
                Container::new()
                    .flex_grow(1.0)
                    .child(Text::new(todo.text).size(15.0).color(text_color)),
            )
            .child(
                Button::new("Delete")
                    .background(Color::from_hex(0x7F1D1D))
                    .text_color(Color::WHITE)
                    .on_click(move || {
                        apply_action(model_for_delete, TodoAction::Delete(id));
                    }),
            )
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

    fn rebuild(&mut self, _ctx: &mut BuildContext) {
        self.rebuild_children();
    }

    fn paint(&self, _ctx: &mut PaintContext) {}

    fn event(&mut self, _ctx: &mut EventContext, _event: &InputEvent) {}

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_model_core_flow() {
        let mut model = TodoModel::default();
        model.apply(TodoAction::SetDraft("Write tests".to_owned()));
        model.apply(TodoAction::AddDraft);
        assert_eq!(model.todos.len(), 1);
        assert_eq!(model.draft, "");
        assert_eq!(model.todos[0].text, "Write tests");

        let id = model.todos[0].id;
        model.apply(TodoAction::Toggle(id));
        assert!(model.todos[0].done);
        model.apply(TodoAction::SetFilter(Filter::Done));
        assert_eq!(model.filtered_todos().count(), 1);
        model.apply(TodoAction::ClearCompleted);
        assert!(model.todos.is_empty());
    }

    #[test]
    fn todo_app_signal_actions_update_state() {
        let runtime = sparsh::signals::RuntimeHandle::new();
        runtime.run_with_current(|| {
            let app = TodoApp::new();
            apply_action(app.model, TodoAction::SetDraft("Alpha".to_owned()));
            apply_action(app.model, TodoAction::AddDraft);

            let snapshot = app.snapshot_model();
            assert_eq!(snapshot.todos.len(), 1);
            assert_eq!(snapshot.todos[0].text, "Alpha");

            let id = snapshot.todos[0].id;
            apply_action(app.model, TodoAction::Toggle(id));
            apply_action(app.model, TodoAction::SetFilter(Filter::Done));

            let updated = app.snapshot_model();
            assert_eq!(updated.filter, Filter::Done);
            assert!(updated.todos[0].done);
        });
    }
}
