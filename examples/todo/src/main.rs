//! Todo example app for Sparsha (native + web) using function components and signals.

use serde_json::json;
use sparsha::prelude::*;

fn main() -> Result<(), sparsha::AppRunError> {
    #[cfg(target_arch = "wasm32")]
    sparsha::init_web()?;

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    let theme_mode = Signal::new(ThemeMode::Light);

    App::new()
        .title("Sparsha Todo")
        .size(960, 720)
        .theme(todo_light_theme())
        .dark_theme(todo_dark_theme())
        .theme_mode(theme_mode)
        .router(
            Router::new()
                .transition(RouterTransition::slide_overlay())
                .route("/", move || {
                    component()
                        .render(move |cx| todo_app(cx, theme_mode))
                        .call()
                })
                .route("/about", || component().render(todo_about).call())
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
    model.with_mut(|state| state.apply(action));
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

fn analysis_summary(task: &TaskHook) -> String {
    let Some(result) = task.result() else {
        return "Background analyzer is idle.".to_owned();
    };

    match result.status {
        TaskStatus::Success(payload) => {
            let words = value_to_u64(payload.get("word_count"));
            let chars = value_to_u64(payload.get("char_count"));
            let lines = value_to_u64(payload.get("line_count"));
            format!(
                "Background analyzer: {} words, {} chars, {} lines",
                words, chars, lines
            )
        }
        TaskStatus::Error(message) => format!("Background analyzer error: {}", message),
        TaskStatus::Canceled => "Background analyzer canceled.".to_owned(),
    }
}

fn toggle_theme_button(theme_mode: Signal<ThemeMode>) -> Button {
    let label = switch_label(theme_mode.get());
    Button::new(label).on_click(move || {
        theme_mode.with_mut(|mode| {
            *mode = toggle_mode(*mode);
        });
    })
}

fn secondary_button(label: &str, on_click: impl FnMut() + 'static) -> Button {
    let theme = current_theme();
    Button::new(label)
        .background(theme.colors.surface_variant)
        .text_color(theme.colors.text_primary)
        .on_click(on_click)
}

fn filter_button(label: &str, model: Signal<TodoModel>, filter: Filter, current: Filter) -> Button {
    let theme = current_theme();
    let selected = current == filter;
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
    Button::new(label)
        .background(background)
        .text_color(text_color)
        .on_click(move || {
            apply_action(model, TodoAction::SetFilter(filter));
        })
}

fn filter_row(model: Signal<TodoModel>, current: Filter) -> Container {
    Container::new()
        .row()
        .fill_width()
        .gap(8.0)
        .child(filter_button("All", model, Filter::All, current))
        .child(filter_button("Active", model, Filter::Active, current))
        .child(filter_button("Done", model, Filter::Done, current))
}

fn clear_completed_button(model: Signal<TodoModel>) -> Button {
    secondary_button("Clear Completed", move || {
        apply_action(model, TodoAction::ClearCompleted);
    })
}

fn about_button(navigator: Navigator) -> Button {
    secondary_button("About", move || {
        navigator.go("/about");
    })
}

fn back_to_todo_button(navigator: Navigator) -> Button {
    secondary_button("Back to Todo", move || {
        navigator.go("/");
    })
}

fn footer_actions(model: Signal<TodoModel>, navigator: Navigator) -> Container {
    Container::new()
        .row()
        .gap(10.0)
        .child(about_button(navigator))
        .child(clear_completed_button(model))
}

fn todo_row(model: Signal<TodoModel>, todo: TodoItem) -> Container {
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
    Container::new()
        .row()
        .fill_width()
        .gap(12.0)
        .padding(12.0)
        .background(row_bg)
        .corner_radius(10.0)
        .align_items(taffy::prelude::AlignItems::Center)
        .child(Checkbox::with_checked(todo.done).on_toggle(move |_| {
            apply_action(model, TodoAction::Toggle(id));
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
                    apply_action(model, TodoAction::Delete(id));
                }),
        )
}

fn input_row(model: Signal<TodoModel>, draft: String, analysis: TaskHook) -> Container {
    let theme = current_theme();
    let model_for_change = model;
    let model_for_submit = model;
    let model_for_add = model;

    Container::new()
        .row()
        .fill_width()
        .gap(10.0)
        .align_items(taffy::prelude::AlignItems::Center)
        .child(
            TextInput::new()
                .value(draft)
                .fill_width()
                .placeholder("Add a task...")
                .on_change(move |value| {
                    apply_action(model_for_change, TodoAction::SetDraft(value.to_owned()));
                    analysis.spawn(json!({ "text": value }));
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

fn todo_app(cx: &mut ComponentContext<'_>, theme_mode: Signal<ThemeMode>) -> Container {
    let model = cx.signal(TodoModel::default());
    let analysis = cx.use_task("todo.input-analysis", "analyze_text");
    let navigator = cx.navigator();
    let snapshot = model.get();
    let theme = cx.theme();
    let is_dark = theme_mode.get() == ThemeMode::Dark;
    let analysis_text = if snapshot.draft.trim().is_empty() {
        String::new()
    } else {
        analysis_summary(&analysis)
    };
    let active_count = snapshot.todos.iter().filter(|todo| !todo.done).count();
    let done_count = snapshot.todos.iter().filter(|todo| todo.done).count();

    let shell_bg = theme.colors.background;
    let panel_bg = theme.colors.surface;
    let card_bg = theme.colors.surface_variant;
    let subdued_text = theme.colors.text_muted;
    let analysis_color = if is_dark {
        theme.colors.border_focus
    } else {
        theme.colors.primary_hovered
    };
    let visible: Vec<TodoItem> = snapshot.filtered_todos().cloned().collect();

    let todo_list = if visible.is_empty() {
        Container::new()
            .padding(14.0)
            .background(card_bg)
            .corner_radius(8.0)
            .child(
                Text::new("No tasks for this filter yet.")
                    .size(14.0)
                    .color(subdued_text),
            )
            .into_widget()
    } else {
        ForEach::new(
            visible,
            |todo| todo.id as usize,
            move |todo| todo_row(model, todo),
        )
        .column()
        .gap(10.0)
        .fill_width()
        .into_widget()
    };

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
                .child(toggle_theme_button(theme_mode)),
        )
        .child(
            Text::new("Sparsha native + web example using signal-driven state.")
                .size(13.0)
                .color(subdued_text),
        )
        .child(Text::new(analysis_text).size(12.0).color(analysis_color))
        .child(input_row(model, snapshot.draft.clone(), analysis))
        .child(filter_row(model, snapshot.filter))
        .child(
            Scroll::new()
                .vertical()
                .fill_width()
                .height(340.0)
                .content(todo_list),
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
                        snapshot.todos.len()
                    ))
                    .size(13.0)
                    .color(subdued_text),
                )
                .child(footer_actions(model, navigator)),
        );

    Container::new()
        .fill()
        .center()
        .background(shell_bg)
        .child(content)
}

fn todo_about(cx: &mut ComponentContext<'_>) -> Container {
    let theme = cx.theme();
    let navigator = cx.navigator();
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
                    Text::new("About Todo")
                        .size(28.0)
                        .bold()
                        .color(theme.colors.text_primary),
                )
                .child(
                    Text::new(
                        "This example stays intentionally small: one task screen, one about route, shared theme state, and the same component code on native and web.",
                    )
                    .size(16.0)
                    .color(theme.colors.text_muted),
                )
                .child(
                    Text::new(
                        "Use the About button in the task footer or switch between `#/` and `#/about` in the browser to verify routing parity.",
                    )
                    .size(14.0)
                    .color(theme.colors.primary),
                )
                .child(back_to_todo_button(navigator)),
        )
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
    fn apply_action_updates_signal_backed_model() {
        let runtime = sparsha::signals::RuntimeHandle::new();
        runtime.run_with_current(|| {
            let model = Signal::new(TodoModel::default());
            apply_action(model, TodoAction::SetDraft("Alpha".to_owned()));
            apply_action(model, TodoAction::AddDraft);

            let snapshot = model.get();
            assert_eq!(snapshot.todos.len(), 1);
            assert_eq!(snapshot.todos[0].text, "Alpha");

            let id = snapshot.todos[0].id;
            apply_action(model, TodoAction::Toggle(id));
            apply_action(model, TodoAction::SetFilter(Filter::Done));

            let updated = model.get();
            assert_eq!(updated.filter, Filter::Done);
            assert!(updated.todos[0].done);
        });
    }
}
