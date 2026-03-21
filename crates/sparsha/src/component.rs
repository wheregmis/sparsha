use crate::{Navigator, TaskKey, TaskPayload, TaskResult, TaskResultSubscription, TaskRuntime};
use sparsha_layout::taffy::prelude::{AlignItems, Display, FlexDirection, Style};
use sparsha_layout::WidgetId;
use sparsha_signals::{Effect, Memo, Signal};
use sparsha_widgets::context::BuildStateStore;
use sparsha_widgets::{current_viewport, BuildContext, IntoWidget, Theme, ViewportInfo, Widget};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Default)]
pub(crate) struct ComponentStateStore {
    states: HashMap<Vec<usize>, Box<dyn Any>>,
    used_paths: HashSet<Vec<usize>>,
}

impl ComponentStateStore {
    pub(crate) fn begin_rebuild(&mut self) {
        self.used_paths.clear();
    }

    pub(crate) fn finish_rebuild(&mut self) {
        self.states.retain(|path, _| self.used_paths.contains(path));
    }
}

impl BuildStateStore for ComponentStateStore {
    fn mark_path_used(&mut self, path: &[usize]) {
        self.used_paths.insert(path.to_vec());
    }

    fn take_boxed_state(&mut self, path: &[usize]) -> Option<Box<dyn Any>> {
        self.states.remove(path)
    }

    fn store_boxed_state(&mut self, path: Vec<usize>, state: Box<dyn Any>) {
        self.states.insert(path, state);
    }
}

#[derive(Default)]
struct StoredComponentState {
    hooks: Vec<Box<dyn Any>>,
    active_hooks: usize,
}

impl StoredComponentState {
    fn hook<T: 'static>(&mut self, index: usize, init: impl FnOnce() -> T) -> &mut T {
        if index == self.hooks.len() {
            self.hooks.push(Box::new(init()));
        }

        self.hooks[index]
            .downcast_mut::<T>()
            .expect("component hook order/type mismatch")
    }
}

struct ManagedEffect(Effect);

impl Drop for ManagedEffect {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

struct TaskHookInner {
    runtime: TaskRuntime,
    task_key: TaskKey,
    task_kind: String,
    generation: Signal<u64>,
    pending: Signal<bool>,
    result: Signal<Option<TaskResult>>,
    _subscription: TaskResultSubscription,
}

struct TaskHookSlot {
    task_key: TaskKey,
    task_kind: String,
    hook: TaskHook,
}

impl TaskHookSlot {
    fn new(runtime: TaskRuntime, task_key: TaskKey, task_kind: String) -> Self {
        let hook = TaskHook::new(runtime, task_key.clone(), task_kind.clone());
        Self {
            task_key,
            task_kind,
            hook,
        }
    }
}

/// A keyed task binding owned by a component hook.
#[derive(Clone)]
pub struct TaskHook {
    inner: Rc<TaskHookInner>,
}

impl TaskHook {
    fn new(runtime: TaskRuntime, task_key: TaskKey, task_kind: String) -> Self {
        let generation = Signal::new(0u64);
        let pending = Signal::new(false);
        let result = Signal::new(None::<TaskResult>);
        let task_key_for_handler = task_key.clone();
        let task_kind_for_handler = task_kind.clone();
        let pending_for_handler = pending;
        let result_for_handler = result;
        let subscription = runtime.on_result(move |task_result| {
            if task_result.task_kind == task_kind_for_handler
                && task_result.task_key.as_ref() == Some(&task_key_for_handler)
            {
                pending_for_handler.set(false);
                result_for_handler.set(Some(task_result));
            }
        });

        Self {
            inner: Rc::new(TaskHookInner {
                runtime,
                task_key,
                task_kind,
                generation,
                pending,
                result,
                _subscription: subscription,
            }),
        }
    }

    pub fn pending(&self) -> bool {
        self.inner.pending.get()
    }

    pub fn result(&self) -> Option<TaskResult> {
        self.inner.result.get()
    }

    pub fn clear(&self) {
        self.inner.pending.set(false);
        self.inner.result.set(None);
    }

    pub fn spawn(&self, payload: TaskPayload) {
        self.inner.pending.set(true);
        self.inner.result.set(None);
        let generation = self.inner.generation.with_mut(|generation| {
            *generation += 1;
            *generation
        });
        self.inner.runtime.spawn_keyed(
            self.inner.task_key.clone(),
            generation,
            self.inner.task_kind.clone(),
            payload,
        );
    }
}

/// Build-time context passed to function components.
pub struct ComponentContext<'a> {
    build: &'a mut BuildContext,
    state: &'a mut StoredComponentState,
    hook_index: usize,
}

impl<'a> ComponentContext<'a> {
    fn new(build: &'a mut BuildContext, state: &'a mut StoredComponentState) -> Self {
        state.active_hooks = 0;
        Self {
            build,
            state,
            hook_index: 0,
        }
    }

    fn next_hook<T: 'static>(&mut self, init: impl FnOnce() -> T) -> &mut T {
        let index = self.hook_index;
        self.hook_index += 1;
        self.state.active_hooks = self.hook_index;
        self.state.hook(index, init)
    }

    pub fn signal<T: 'static>(&mut self, initial: T) -> Signal<T> {
        *self.next_hook(|| Signal::new(initial))
    }

    pub fn memo<T: Clone + 'static>(&mut self, compute: impl FnMut() -> T + 'static) -> Memo<T> {
        *self.next_hook(|| Memo::new(compute))
    }

    pub fn effect(&mut self, callback: impl FnMut() + 'static) -> Effect {
        self.next_hook(|| ManagedEffect(Effect::new(callback))).0
    }

    pub fn theme(&self) -> Theme {
        self.build.theme()
    }

    pub fn viewport(&self) -> ViewportInfo {
        self.build
            .resource::<ViewportInfo>()
            .unwrap_or_else(current_viewport)
    }

    pub fn navigator(&self) -> Navigator {
        self.build
            .resource::<Navigator>()
            .expect("component navigator resource unavailable")
    }

    pub fn task_runtime(&self) -> TaskRuntime {
        self.build
            .resource::<TaskRuntime>()
            .unwrap_or_else(TaskRuntime::current_or_default)
    }

    pub fn use_task(
        &mut self,
        task_key: impl Into<TaskKey>,
        task_kind: impl Into<String>,
    ) -> TaskHook {
        let runtime = self.task_runtime();
        let task_key = task_key.into();
        let task_kind = task_kind.into();
        let slot = self
            .next_hook(|| TaskHookSlot::new(runtime.clone(), task_key.clone(), task_kind.clone()));
        if slot.task_key != task_key || slot.task_kind != task_kind {
            *slot = TaskHookSlot::new(runtime, task_key, task_kind);
        }
        slot.hook.clone()
    }
}

/// A function component host widget.
pub struct Component<F> {
    id: WidgetId,
    render: F,
    children: Vec<Box<dyn Widget>>,
}

impl<F> Component<F> {
    fn new(render: F) -> Self {
        Self {
            id: WidgetId::default(),
            render,
            children: Vec::new(),
        }
    }
}

/// Build a widget from a function component.
pub fn component<F, W>(render: F) -> Component<F>
where
    F: for<'a> Fn(&'a mut ComponentContext<'a>) -> W + 'static,
    W: IntoWidget,
{
    Component::new(render)
}

impl<F, W> Widget for Component<F>
where
    F: for<'a> Fn(&'a mut ComponentContext<'a>) -> W + 'static,
    W: IntoWidget,
{
    fn id(&self) -> WidgetId {
        self.id
    }

    fn set_id(&mut self, id: WidgetId) {
        self.id = id;
    }

    fn style(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: Some(AlignItems::Stretch),
            ..Default::default()
        }
    }

    fn rebuild(&mut self, ctx: &mut BuildContext) {
        let mut state = ctx
            .take_boxed_state()
            .and_then(|state| state.downcast::<StoredComponentState>().ok())
            .map(|state| *state)
            .unwrap_or_default();

        let child = {
            let mut component_ctx = ComponentContext::new(ctx, &mut state);
            (self.render)(&mut component_ctx).into_widget()
        };
        state.hooks.truncate(state.active_hooks);

        self.children = vec![child];
        ctx.store_boxed_state(Box::new(state));
    }

    fn paint(&self, _ctx: &mut sparsha_widgets::PaintContext) {}

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
    use crate::TaskStatus;
    use serde_json::json;
    use sparsha_signals::RuntimeHandle;
    use sparsha_widgets::Text;

    #[test]
    fn component_signal_state_survives_host_recreation_at_same_path() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let observed = Signal::new(0usize);
            let mut store = ComponentStateStore::default();

            for expected in [0usize, 1usize] {
                let mut build = BuildContext::default();
                build.set_path(&[0]);
                // SAFETY: the test owns `store` for the full rebuild and does
                // not alias it while `build` is using it.
                unsafe { build.set_state_store(&mut store) };
                let mut host = component(move |cx| {
                    let counter = cx.signal(0usize);
                    observed.set(counter.get());
                    counter.set(counter.get() + 1);
                    Text::new("component")
                });
                host.rebuild(&mut build);
                assert_eq!(observed.get(), expected);
            }
        });
    }

    #[test]
    fn task_hook_tracks_latest_result() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let task_runtime = TaskRuntime::new();
            let hook_slot = Signal::new(None::<TaskHook>);
            let mut store = ComponentStateStore::default();

            let mut build = BuildContext::default();
            build.set_path(&[0]);
            build.insert_resource(task_runtime.clone());
            // SAFETY: the test owns `store` for the full rebuild and does not
            // alias it while `build` is using it.
            unsafe { build.set_state_store(&mut store) };

            let mut host = component(move |cx| {
                hook_slot.set(Some(cx.use_task("component.test", "echo")));
                Text::new("task")
            });
            host.rebuild(&mut build);

            let hook = hook_slot.get().expect("task hook");
            hook.spawn(json!({"text": "hello"}));

            let mut received = Vec::new();
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1600);
            while std::time::Instant::now() < deadline {
                task_runtime.drain_completed(|result| received.push(result));
                if hook.result().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let result = hook.result().expect("completed task result");
            assert_eq!(result.task_kind, "echo");
            assert!(matches!(result.status, TaskStatus::Success(_)));
            assert!(!hook.pending());
            assert!(!received.is_empty());
        });
    }

    #[test]
    fn task_hook_rebinds_when_key_changes() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let task_runtime = TaskRuntime::new();
            let task_key = Signal::new(TaskKey::new("todos.a"));
            let hook_slot = Signal::new(None::<TaskHook>);
            let mut store = ComponentStateStore::default();

            let mut build = BuildContext::default();
            build.set_path(&[0]);
            build.insert_resource(task_runtime.clone());
            // SAFETY: the test owns `store` for the full rebuild and does not
            // alias it while `build` is using it.
            unsafe { build.set_state_store(&mut store) };

            let mut host = component(move |cx| {
                hook_slot.set(Some(cx.use_task(task_key.get(), "echo")));
                Text::new("task")
            });
            host.rebuild(&mut build);

            task_key.set(TaskKey::new("todos.b"));
            host.rebuild(&mut build);

            let hook = hook_slot.get().expect("task hook");
            hook.spawn(json!({"text": "hello"}));

            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1600);
            while std::time::Instant::now() < deadline {
                task_runtime.drain_completed(|_| {});
                if hook.result().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let result = hook.result().expect("completed task result");
            assert_eq!(result.task_key, Some(TaskKey::new("todos.b")));
        });
    }

    #[test]
    fn component_context_exposes_viewport_resource() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let observed = Signal::new(None::<ViewportInfo>);
            let mut store = ComponentStateStore::default();

            let mut build = BuildContext::default();
            build.set_path(&[0]);
            build.insert_resource(ViewportInfo::new(820.0, 1180.0));
            unsafe { build.set_state_store(&mut store) };

            let mut host = component(move |cx| {
                observed.set(Some(cx.viewport()));
                Text::new("viewport")
            });
            host.rebuild(&mut build);

            let viewport = observed.get().expect("viewport");
            assert_eq!(viewport.width, 820.0);
            assert_eq!(viewport.class, sparsha_widgets::ViewportClass::Tablet);
        });
    }
}
