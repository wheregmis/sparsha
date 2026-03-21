//! Reactive signals runtime for Sparsh.
//!
//! Stability: the supported 1.0 contract is the public signal/runtime API defined at this crate
//! root.

use generational_box::{AnyStorage, GenerationalBox, Owner, UnsyncStorage};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SubscriberKind {
    Rebuild,
    Layout,
    Paint,
    Effect,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DirtyFlags {
    pub rebuild: bool,
    pub layout: bool,
    pub paint: bool,
}

impl DirtyFlags {
    pub fn any(self) -> bool {
        self.rebuild || self.layout || self.paint
    }
}

type RuntimeId = u64;
type SubscriberId = u64;
type SignalId = u64;

type EffectCallback = Rc<RefCell<dyn FnMut()>>;

struct RuntimeInner {
    owner: Owner<UnsyncStorage>,
    next_signal_id: SignalId,
    next_subscriber_id: SubscriberId,
    phase_subscribers: HashMap<SubscriberKind, SubscriberId>,
    subscribers: HashMap<SubscriberId, SubscriberKind>,
    signal_subscribers: HashMap<SignalId, HashSet<SubscriberId>>,
    subscriber_signals: HashMap<SubscriberId, HashSet<SignalId>>,
    effects: HashMap<SubscriberId, EffectCallback>,
    pending_effects: VecDeque<SubscriberId>,
    pending_effect_set: HashSet<SubscriberId>,
    dirty: DirtyFlags,
    scheduler: Option<Box<dyn FnMut()>>,
}

impl RuntimeInner {
    fn new() -> Self {
        Self {
            owner: UnsyncStorage::owner(),
            next_signal_id: 1,
            next_subscriber_id: 1,
            phase_subscribers: HashMap::new(),
            subscribers: HashMap::new(),
            signal_subscribers: HashMap::new(),
            subscriber_signals: HashMap::new(),
            effects: HashMap::new(),
            pending_effects: VecDeque::new(),
            pending_effect_set: HashSet::new(),
            dirty: DirtyFlags::default(),
            scheduler: None,
        }
    }

    fn new_subscriber(&mut self, kind: SubscriberKind) -> SubscriberId {
        let id = self.next_subscriber_id;
        self.next_subscriber_id += 1;
        self.subscribers.insert(id, kind);
        id
    }

    fn phase_subscriber(&mut self, kind: SubscriberKind) -> SubscriberId {
        if let Some(id) = self.phase_subscribers.get(&kind) {
            *id
        } else {
            let id = self.new_subscriber(kind);
            self.phase_subscribers.insert(kind, id);
            id
        }
    }

    fn clear_subscriber_dependencies(&mut self, subscriber: SubscriberId) {
        if let Some(signals) = self.subscriber_signals.remove(&subscriber) {
            for signal_id in signals {
                if let Some(subscribers) = self.signal_subscribers.get_mut(&signal_id) {
                    subscribers.remove(&subscriber);
                    if subscribers.is_empty() {
                        self.signal_subscribers.remove(&signal_id);
                    }
                }
            }
        }
    }

    fn remove_subscriber(&mut self, subscriber: SubscriberId) {
        self.clear_subscriber_dependencies(subscriber);
        self.subscribers.remove(&subscriber);
        self.effects.remove(&subscriber);
        self.pending_effect_set.remove(&subscriber);
        self.pending_effects
            .retain(|candidate| *candidate != subscriber);
        self.phase_subscribers.retain(|_, id| *id != subscriber);
    }

    fn track_read(&mut self, subscriber: SubscriberId, signal_id: SignalId) {
        self.signal_subscribers
            .entry(signal_id)
            .or_default()
            .insert(subscriber);
        self.subscriber_signals
            .entry(subscriber)
            .or_default()
            .insert(signal_id);
    }

    fn notify_write(&mut self, signal_id: SignalId) {
        let mut should_schedule = false;
        if let Some(subscribers) = self.signal_subscribers.get(&signal_id).cloned() {
            for subscriber in subscribers {
                match self.subscribers.get(&subscriber).copied() {
                    Some(SubscriberKind::Rebuild) => {
                        self.dirty.rebuild = true;
                        self.dirty.layout = true;
                        self.dirty.paint = true;
                        should_schedule = true;
                    }
                    Some(SubscriberKind::Layout) => {
                        self.dirty.layout = true;
                        self.dirty.paint = true;
                        should_schedule = true;
                    }
                    Some(SubscriberKind::Paint) => {
                        self.dirty.paint = true;
                        should_schedule = true;
                    }
                    Some(SubscriberKind::Effect) => {
                        if self.pending_effect_set.insert(subscriber) {
                            self.pending_effects.push_back(subscriber);
                        }
                        should_schedule = true;
                    }
                    None => {}
                }
            }
        }

        if should_schedule {
            if let Some(callback) = &mut self.scheduler {
                callback();
            }
        }
    }

    fn take_dirty(&mut self) -> DirtyFlags {
        let dirty = self.dirty;
        self.dirty = DirtyFlags::default();
        dirty
    }
}

#[derive(Clone)]
pub struct RuntimeHandle {
    id: RuntimeId,
    inner: Rc<RefCell<RuntimeInner>>,
}

thread_local! {
    static NEXT_RUNTIME_ID: Cell<u64> = const { Cell::new(1) };
    static REGISTRY: RefCell<HashMap<RuntimeId, Weak<RefCell<RuntimeInner>>>> = RefCell::new(HashMap::new());
    static CURRENT_RUNTIME: Cell<Option<RuntimeId>> = const { Cell::new(None) };
    static CURRENT_SUBSCRIBER: Cell<Option<SubscriberId>> = const { Cell::new(None) };
    static DEFAULT_RUNTIME: RefCell<Option<RuntimeHandle>> = const { RefCell::new(None) };
}

impl RuntimeHandle {
    pub fn new() -> Self {
        let id = NEXT_RUNTIME_ID.with(|cell| {
            let id = cell.get();
            cell.set(id + 1);
            id
        });
        let inner = Rc::new(RefCell::new(RuntimeInner::new()));
        REGISTRY.with(|registry| {
            registry.borrow_mut().insert(id, Rc::downgrade(&inner));
        });
        Self { id, inner }
    }

    pub fn current_or_default() -> Self {
        if let Some(runtime) = Self::current() {
            return runtime;
        }

        DEFAULT_RUNTIME.with(|slot| {
            if let Some(existing) = slot.borrow().as_ref() {
                return existing.clone();
            }
            let runtime = Self::new();
            *slot.borrow_mut() = Some(runtime.clone());
            runtime
        })
    }

    pub fn current() -> Option<Self> {
        let id = CURRENT_RUNTIME.with(Cell::get)?;
        let inner = REGISTRY.with(|registry| registry.borrow().get(&id).and_then(Weak::upgrade))?;
        Some(Self { id, inner })
    }

    pub fn run_with_current<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = CURRENT_RUNTIME.with(|cell| {
            let prev = cell.get();
            cell.set(Some(self.id));
            prev
        });
        let out = f();
        CURRENT_RUNTIME.with(|cell| cell.set(previous));
        out
    }

    pub fn set_scheduler(&self, callback: impl FnMut() + 'static) {
        self.inner.borrow_mut().scheduler = Some(Box::new(callback));
    }

    pub fn clear_scheduler(&self) {
        self.inner.borrow_mut().scheduler = None;
    }

    pub fn with_tracking<R>(&self, kind: SubscriberKind, f: impl FnOnce() -> R) -> R {
        let subscriber = {
            let mut inner = self.inner.borrow_mut();
            let subscriber = inner.phase_subscriber(kind);
            inner.clear_subscriber_dependencies(subscriber);
            subscriber
        };

        let prev_runtime = CURRENT_RUNTIME.with(|cell| {
            let prev = cell.get();
            cell.set(Some(self.id));
            prev
        });
        let prev_subscriber = CURRENT_SUBSCRIBER.with(|cell| {
            let prev = cell.get();
            cell.set(Some(subscriber));
            prev
        });

        let out = f();

        CURRENT_SUBSCRIBER.with(|cell| cell.set(prev_subscriber));
        CURRENT_RUNTIME.with(|cell| cell.set(prev_runtime));

        out
    }

    pub fn register_effect(&self, callback: impl FnMut() + 'static) -> Effect {
        let subscriber = {
            let mut inner = self.inner.borrow_mut();
            let subscriber = inner.new_subscriber(SubscriberKind::Effect);
            inner
                .effects
                .insert(subscriber, Rc::new(RefCell::new(callback)));
            subscriber
        };

        self.run_effect_subscriber(subscriber);

        Effect {
            runtime_id: self.id,
            subscriber,
        }
    }

    fn run_effect_subscriber(&self, subscriber: SubscriberId) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.clear_subscriber_dependencies(subscriber);
        }

        let callback = {
            let inner = self.inner.borrow();
            inner.effects.get(&subscriber).cloned()
        };

        if let Some(callback) = callback {
            let prev_runtime = CURRENT_RUNTIME.with(|cell| {
                let prev = cell.get();
                cell.set(Some(self.id));
                prev
            });
            let prev_subscriber = CURRENT_SUBSCRIBER.with(|cell| {
                let prev = cell.get();
                cell.set(Some(subscriber));
                prev
            });

            (callback.borrow_mut())();

            CURRENT_SUBSCRIBER.with(|cell| cell.set(prev_subscriber));
            CURRENT_RUNTIME.with(|cell| cell.set(prev_runtime));
        }
    }

    pub fn run_effects(&self, max_iterations: usize) {
        let max = max_iterations.max(1);
        for _ in 0..max {
            let next_effect = {
                let mut inner = self.inner.borrow_mut();
                let effect = inner.pending_effects.pop_front();
                if let Some(id) = effect {
                    inner.pending_effect_set.remove(&id);
                }
                effect
            };

            let Some(effect_id) = next_effect else {
                break;
            };

            self.run_effect_subscriber(effect_id);
        }
    }

    pub fn take_dirty_flags(&self) -> DirtyFlags {
        self.inner.borrow_mut().take_dirty()
    }

    fn new_signal<T: 'static>(&self, value: T) -> Signal<T> {
        let (signal_id, slot) = {
            let mut inner = self.inner.borrow_mut();
            let id = inner.next_signal_id;
            inner.next_signal_id += 1;
            (id, inner.owner.insert(value))
        };

        Signal {
            runtime_id: self.id,
            signal_id,
            slot,
        }
    }
}

impl Default for RuntimeHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Signal<T: 'static> {
    runtime_id: RuntimeId,
    signal_id: SignalId,
    slot: GenerationalBox<T>,
}

impl<T: 'static> Copy for Signal<T> {}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        RuntimeHandle::current_or_default().new_signal(value)
    }

    pub fn split(self) -> (ReadSignal<T>, WriteSignal<T>) {
        (ReadSignal { inner: self }, WriteSignal { inner: self })
    }

    pub fn read_only(self) -> ReadSignal<T> {
        ReadSignal { inner: self }
    }

    pub fn write_only(self) -> WriteSignal<T> {
        WriteSignal { inner: self }
    }

    fn runtime(&self) -> Option<Rc<RefCell<RuntimeInner>>> {
        REGISTRY.with(|registry| {
            registry
                .borrow()
                .get(&self.runtime_id)
                .and_then(Weak::upgrade)
        })
    }

    fn track_read(&self) {
        let active_runtime = CURRENT_RUNTIME.with(Cell::get);
        let active_subscriber = CURRENT_SUBSCRIBER.with(Cell::get);
        if active_runtime != Some(self.runtime_id) {
            return;
        }
        let Some(subscriber) = active_subscriber else {
            return;
        };
        if let Some(runtime) = self.runtime() {
            runtime.borrow_mut().track_read(subscriber, self.signal_id);
        }
    }

    fn track_write(&self) {
        if let Some(runtime) = self.runtime() {
            runtime.borrow_mut().notify_write(self.signal_id);
        }
    }

    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.track_read();
        let value = self.slot.read();
        f(&value)
    }

    pub fn with_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut value = self.slot.write();
        let out = f(&mut value);
        drop(value);
        self.track_write();
        out
    }

    pub fn set(&self, value: T) {
        *self.slot.write() = value;
        self.track_write();
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        self.with(Clone::clone)
    }
}

pub struct ReadSignal<T: 'static> {
    inner: Signal<T>,
}

impl<T: 'static> Copy for ReadSignal<T> {}

impl<T: 'static> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Clone + 'static> ReadSignal<T> {
    pub fn get(&self) -> T {
        self.inner.get()
    }
}

impl<T: 'static> ReadSignal<T> {
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        self.inner.with(f)
    }
}

pub struct WriteSignal<T: 'static> {
    inner: Signal<T>,
}

impl<T: 'static> Copy for WriteSignal<T> {}

impl<T: 'static> Clone for WriteSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> WriteSignal<T> {
    pub fn set(&self, value: T) {
        self.inner.set(value);
    }

    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.inner.with_mut(f)
    }
}

pub struct Memo<T: 'static> {
    signal: Signal<T>,
}

impl<T: 'static> Copy for Memo<T> {}

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Clone + 'static> Memo<T> {
    pub fn new(mut compute: impl FnMut() -> T + 'static) -> Self {
        let signal = Signal::new(compute());
        let output = signal;
        Effect::new(move || {
            output.set(compute());
        });
        Self { signal }
    }

    pub fn get(&self) -> T {
        self.signal.get()
    }

    pub fn read_only(self) -> ReadSignal<T> {
        self.signal.read_only()
    }
}

pub struct Effect {
    runtime_id: RuntimeId,
    subscriber: SubscriberId,
}

impl Copy for Effect {}

impl Clone for Effect {
    fn clone(&self) -> Self {
        *self
    }
}

impl Effect {
    pub fn new(callback: impl FnMut() + 'static) -> Self {
        RuntimeHandle::current_or_default().register_effect(callback)
    }

    pub fn id(self) -> u64 {
        self.subscriber
    }

    pub fn runtime_id(self) -> u64 {
        self.runtime_id
    }

    pub fn dispose(self) {
        let runtime = REGISTRY.with(|registry| {
            registry
                .borrow()
                .get(&self.runtime_id)
                .and_then(Weak::upgrade)
        });
        if let Some(runtime) = runtime {
            runtime.borrow_mut().remove_subscriber(self.subscriber);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_tracks_phase_dependencies_and_marks_dirty() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let value = Signal::new(1usize);

            runtime.with_tracking(SubscriberKind::Paint, || {
                assert_eq!(value.get(), 1);
            });

            value.set(2);
            let dirty = runtime.take_dirty_flags();
            assert!(dirty.paint);
            assert!(!dirty.layout);
            assert!(!dirty.rebuild);
        });
    }

    #[test]
    fn memo_recomputes_when_dependency_changes() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let base = Signal::new(2i32);
            let memo = Memo::new(move || base.get() * 3);

            assert_eq!(memo.get(), 6);
            base.set(5);
            runtime.run_effects(16);
            assert_eq!(memo.get(), 15);
        });
    }

    #[test]
    fn effect_runs_on_signal_change() {
        let runtime = RuntimeHandle::new();
        runtime.run_with_current(|| {
            let value = Signal::new(0i32);
            let observed = Signal::new(0i32);

            let _effect = Effect::new(move || {
                observed.set(value.get());
            });

            value.set(7);
            runtime.run_effects(16);
            assert_eq!(observed.get(), 7);
        });
    }
}
