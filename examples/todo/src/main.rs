//! Todo example app for Sparsh (native + web) using signal-based state.

use serde_json::json;
use sparsh::prelude::*;
use sparsh::widgets::{current_theme, BuildContext, EventContext, PaintContext, WidgetId};

fn main() -> Result<(), sparsh::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsh::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let theme_mode = Signal::new(ThemeMode::Light);

    App::new()
        .title("Sparsh Todo")
        .size(960, 720)
        .theme(todo_light_theme())
        .dark_theme(todo_dark_theme())
        .theme_mode(theme_mode)
        .router(
            Router::new()
                .route("/", move || Box::new(TodoApp::new(theme_mode)))
                .route("/about", || Box::new(TodoAbout::new()))
                .fallback("/"),
        )
        .run()
}

fn toggle_mode(mode: ThemeMode) -> ThemeMode {
    match mode {
        ThemeMode::Light => ThemeMode::Dark,
        ThemeMode::Dark => ThemeMode::Light,
    }
}

fn switch_label(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Light => "Switch to Dark",
        ThemeMode::Dark => "Switch to Light",
    }
}

fn apply_todo_brand(mut theme: Theme) -> Theme {
    theme.colors.primary = Color::from_hex(0x2563EB);
    theme.colors.primary_hovered = Color::from_hex(0x1D4ED8);
    theme.colors.primary_pressed = Color::from_hex(0x1E40AF);
    theme
}

fn todo_light_theme() -> Theme {
    apply_todo_brand(Theme::light())
}

fn todo_dark_theme() -> Theme {
    apply_todo_brand(Theme::dark())
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
    theme_mode: Signal<ThemeMode>,
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
    fn new(theme_mode: Signal<ThemeMode>) -> Self {
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
            theme_mode,
            analysis_text,
            analysis_generation: Signal::new(0),
            task_runtime,
            children: Vec::new(),
        };
        app.rebuild_children();
        app
    }

    fn toggle_theme_button(&self) -> Button {
        let theme_mode = self.theme_mode;
        let label = switch_label(theme_mode.get());

        Button::new(label).on_click(move || {
            theme_mode.with_mut(|mode| {
                *mode = toggle_mode(*mode);
            });
        })
    }

    #[cfg(test)]
    fn snapshot_model(&self) -> TodoModel {
        self.model.get()
    }

    fn rebuild_children(&mut self) {
        let model = self.model.get();
        let theme = current_theme();
        let is_dark = self.theme_mode.get() == ThemeMode::Dark;
        let analysis_text = self.analysis_text.get();
        let active_count = model.todos.iter().filter(|todo| !todo.done).count();
        let done_count = model.todos.iter().filter(|todo| todo.done).count();

        let shell_bg = theme.colors.background;
        let panel_bg = theme.colors.surface;
        let card_bg = theme.colors.surface_variant;
        let subdued_text = theme.colors.text_muted;
        let analysis_color = if is_dark {
            theme.colors.border_focus
        } else {
            theme.colors.primary_hovered
        };

        let mut list = List::new().vertical().gap(10.0).fill_width();
        let visible: Vec<TodoItem> = model.filtered_todos().cloned().collect();
        if visible.is_empty() {
            list.push_item(
                Container::new()
                    .padding(14.0)
                    .background(card_bg)
                    .corner_radius(8.0)
                    .child(
                        Text::new("No tasks for this filter yet.")
                            .size(14.0)
                            .color(subdued_text),
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
            .background(panel_bg)
            .corner_radius(14.0)
            .child(
                Container::new()
                    .row()
                    .fill_width()
                    .space_between()
                    .align_items(taffy::prelude::AlignItems::Center)
                    .child(
                        Text::new("Todo")
                            .size(28.0)
                            .bold()
                            .color(theme.colors.text_primary),
                    )
                    .child(self.toggle_theme_button()),
            )
            .child(
                Text::new("Sparsh native + web example using signal-driven state.")
                    .size(13.0)
                    .color(subdued_text),
            )
            .child(Text::new(analysis_text).size(12.0).color(analysis_color))
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
                        .color(subdued_text),
                    )
                    .child(self.clear_completed_button()),
            );

        self.children = vec![Box::new(
            Container::new()
                .fill()
                .center()
                .background(shell_bg)
                .child(content),
        )];
    }

    fn input_row(&self, model: &TodoModel) -> Container {
        let theme = current_theme();
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
                    .background(theme.colors.primary)
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
        let theme = current_theme();
        let selected = current_filter == filter;
        let background = if selected {
            theme.colors.primary
        } else {
            theme.colors.surface_variant
        };
        let text_color = if selected {
            Color::WHITE
        } else {
            theme.colors.text_primary
        };
        let model = self.model;
        Button::new(label)
            .background(background)
            .text_color(text_color)
            .on_click(move || {
                apply_action(model, TodoAction::SetFilter(filter));
            })
    }

    fn clear_completed_button(&self) -> Button {
        let theme = current_theme();
        let model = self.model;
        Button::new("Clear Completed")
            .background(theme.colors.surface_variant)
            .text_color(theme.colors.text_primary)
            .on_click(move || {
                apply_action(model, TodoAction::ClearCompleted);
            })
    }

    fn todo_row(&self, todo: TodoItem) -> Container {
        let theme = current_theme();
        let text_color = if todo.done {
            theme.colors.text_muted
        } else {
            theme.colors.text_primary
        };
        let row_bg = if todo.done {
            theme.colors.surface_done
        } else {
            theme.colors.surface_variant
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
                    .background(theme.colors.error)
                    .text_color(Color::WHITE)
                    .on_click(move || {
                        apply_action(model_for_delete, TodoAction::Delete(id));
                    }),
            )
    }
}

struct TodoAbout {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl TodoAbout {
    fn new() -> Self {
        let mut page = Self {
            id: WidgetId::default(),
            children: Vec::new(),
        };
        page.rebuild_children();
        page
    }

    fn rebuild_children(&mut self) {
        let theme = current_theme();
        self.children = vec![Box::new(
            Container::new()
                .fill()
                .padding(32.0)
                .background(theme.colors.background)
                .child(
                    Container::new()
                        .column()
                        .gap(16.0)
                        .padding(24.0)
                        .background(theme.colors.surface)
                        .corner_radius(16.0)
                        .child(
                            Text::new("Todo Route Demo")
                                .size(28.0)
                                .bold()
                                .color(theme.colors.text_primary),
                        )
                        .child(
                            Text::new(
                                "This static route exists so the web runtime can exercise hash navigation and route rebuilding on the same widget tree as the main todo app.",
                            )
                            .size(16.0)
                            .color(theme.colors.text_muted),
                        )
                        .child(
                            Text::new("Switch between `#/` and `#/about` in the browser to verify web routing parity.")
                                .size(14.0)
                                .color(theme.colors.primary),
                        ),
                ),
        )];
    }
}

impl Widget for TodoAbout {
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

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children
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
            let app = TodoApp::new(Signal::new(ThemeMode::Light));
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
