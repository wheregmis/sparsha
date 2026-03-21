//! Cross-platform background task runtime.
//!
//! Native uses Tokio worker threads. Web uses dedicated Web Workers.

use serde::{Deserialize, Serialize};
#[cfg(any(not(target_arch = "wasm32"), all(test, not(target_arch = "wasm32"))))]
use serde_json::json;
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc, Mutex, MutexGuard,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};

pub type TaskId = u64;
pub type Generation = u64;
pub type TaskPayload = Value;

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static CURRENT_TASK_RUNTIME: RefCell<Option<TaskRuntime>> = const { RefCell::new(None) };
    static RESULT_HANDLERS: RefCell<HashMap<u64, Vec<ResultHandler>>> = RefCell::new(HashMap::new());
}

type ResultHandler = Box<dyn FnMut(TaskResult)>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskRuntimeInitError {
    NativeRuntime(String),
}

impl core::fmt::Display for TaskRuntimeInitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NativeRuntime(message) => {
                write!(f, "failed to initialize native task runtime: {message}")
            }
        }
    }
}

impl std::error::Error for TaskRuntimeInitError {}

fn lock_recover<'a, T>(mutex: &'a Mutex<T>, label: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("recovering from poisoned mutex: {label}");
            poisoned.into_inner()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskKey(pub String);

impl TaskKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for TaskKey {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for TaskKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskPolicy {
    #[default]
    LatestWins,
    KeepAll,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Success(TaskPayload),
    Error(String),
    Canceled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub task_kind: String,
    pub task_key: Option<TaskKey>,
    pub generation: Option<Generation>,
    pub status: TaskStatus,
}

impl TaskResult {
    fn canceled(task_id: TaskId, meta: &TaskMeta) -> Self {
        Self {
            task_id,
            task_kind: meta.task_kind.clone(),
            task_key: meta.task_key.clone(),
            generation: meta.generation,
            status: TaskStatus::Canceled,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskHandle {
    id: TaskId,
}

impl TaskHandle {
    pub fn id(self) -> TaskId {
        self.id
    }
}

#[derive(Clone)]
pub struct TaskRuntime {
    id: u64,
    inner: Arc<Inner>,
}

impl Default for TaskRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskRuntime {
    pub fn new() -> Self {
        match Self::try_new() {
            Ok(runtime) => runtime,
            Err(err) => {
                log::warn!("task runtime unavailable, using disabled runtime: {err}");
                let runtime = Self::disabled(err.to_string());
                CURRENT_TASK_RUNTIME.with(|slot| {
                    if slot.borrow().is_none() {
                        *slot.borrow_mut() = Some(runtime.clone());
                    }
                });
                runtime
            }
        }
    }

    pub fn try_new() -> Result<Self, TaskRuntimeInitError> {
        let (completion_tx, completion_rx) = mpsc::channel();

        let runtime = Self {
            id: NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed),
            inner: Arc::new(Inner {
                policy: Mutex::new(TaskPolicy::LatestWins),
                next_task_id: AtomicU64::new(1),
                in_flight: AtomicUsize::new(0),
                latest_generation: Mutex::new(HashMap::new()),
                task_meta: Mutex::new(HashMap::new()),
                completion_tx,
                completion_rx: Mutex::new(completion_rx),
                disabled_reason: None,
                #[cfg(not(target_arch = "wasm32"))]
                native: Some(build_native_state()?),
                #[cfg(target_arch = "wasm32")]
                web: Some(Mutex::new(WebState {
                    worker_script_url: "sparsh-worker.js".to_owned(),
                    workers: Vec::new(),
                    next_worker: 0,
                    next_worker_token: 1,
                    task_workers: HashMap::new(),
                })),
            }),
        };

        CURRENT_TASK_RUNTIME.with(|slot| {
            if slot.borrow().is_none() {
                *slot.borrow_mut() = Some(runtime.clone());
            }
        });

        Ok(runtime)
    }

    fn disabled(reason: impl Into<String>) -> Self {
        let (completion_tx, completion_rx) = mpsc::channel();
        Self {
            id: NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed),
            inner: Arc::new(Inner {
                policy: Mutex::new(TaskPolicy::LatestWins),
                next_task_id: AtomicU64::new(1),
                in_flight: AtomicUsize::new(0),
                latest_generation: Mutex::new(HashMap::new()),
                task_meta: Mutex::new(HashMap::new()),
                completion_tx,
                completion_rx: Mutex::new(completion_rx),
                disabled_reason: Some(reason.into()),
                #[cfg(not(target_arch = "wasm32"))]
                native: None,
                #[cfg(target_arch = "wasm32")]
                web: None,
            }),
        }
    }

    pub fn current_or_default() -> Self {
        CURRENT_TASK_RUNTIME.with(|slot| {
            if let Some(existing) = slot.borrow().as_ref() {
                return existing.clone();
            }
            let runtime = Self::new();
            *slot.borrow_mut() = Some(runtime.clone());
            runtime
        })
    }

    pub fn set_current(&self) {
        CURRENT_TASK_RUNTIME.with(|slot| {
            *slot.borrow_mut() = Some(self.clone());
        });
    }

    pub fn policy(&self) -> TaskPolicy {
        *lock_recover(&self.inner.policy, "task policy")
    }

    pub fn set_policy(&self, policy: TaskPolicy) {
        *lock_recover(&self.inner.policy, "task policy") = policy;
    }

    pub fn has_in_flight(&self) -> bool {
        self.inner.in_flight.load(Ordering::Relaxed) > 0
    }

    pub fn on_result(&self, handler: impl FnMut(TaskResult) + 'static) {
        RESULT_HANDLERS.with(|handlers| {
            handlers
                .borrow_mut()
                .entry(self.id)
                .or_default()
                .push(Box::new(handler));
        });
    }

    pub fn spawn(&self, task_kind: impl Into<String>, payload: TaskPayload) -> TaskHandle {
        self.spawn_internal(task_kind.into(), payload, None, None)
    }

    pub fn spawn_keyed(
        &self,
        task_key: impl Into<TaskKey>,
        generation: Generation,
        task_kind: impl Into<String>,
        payload: TaskPayload,
    ) -> TaskHandle {
        let task_key = task_key.into();
        {
            let mut latest = lock_recover(&self.inner.latest_generation, "task latest_generation");
            latest
                .entry(task_key.clone())
                .and_modify(|existing| {
                    *existing = (*existing).max(generation);
                })
                .or_insert(generation);
        }
        self.spawn_internal(task_kind.into(), payload, Some(task_key), Some(generation))
    }

    pub fn cancel(&self, task_id: TaskId) -> bool {
        let meta = {
            let task_meta = lock_recover(&self.inner.task_meta, "task metadata");
            task_meta.get(&task_id).cloned()
        };
        let Some(meta) = meta else {
            return false;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(native) = self.inner.native.as_ref() {
                if let Some(handle) = lock_recover(&native.handles, "task handles").remove(&task_id)
                {
                    handle.abort();
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.send_web_cancel(task_id);
        }

        if !self.try_finish_task(task_id) {
            return false;
        }

        let _ = self
            .inner
            .completion_tx
            .send(TaskResult::canceled(task_id, &meta));
        true
    }

    pub fn drain_completed<F>(&self, mut on_result: F) -> usize
    where
        F: FnMut(TaskResult),
    {
        let mut delivered = 0;

        loop {
            let next = {
                let receiver = lock_recover(&self.inner.completion_rx, "task completion receiver");
                receiver.try_recv()
            };

            let Ok(result) = next else {
                break;
            };

            if self.is_stale(&result) {
                continue;
            }

            on_result(result.clone());
            self.notify_handlers(result);
            delivered += 1;
        }

        delivered
    }

    fn spawn_internal(
        &self,
        task_kind: String,
        payload: TaskPayload,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
    ) -> TaskHandle {
        let task_id = self.inner.next_task_id.fetch_add(1, Ordering::Relaxed);
        if let Some(reason) = self.inner.disabled_reason.as_deref() {
            let _ = self.inner.completion_tx.send(TaskResult {
                task_id,
                task_kind,
                task_key,
                generation,
                status: TaskStatus::Error(format!("task runtime unavailable: {reason}")),
            });
            return TaskHandle { id: task_id };
        }

        let meta = TaskMeta {
            task_kind: task_kind.clone(),
            task_key: task_key.clone(),
            generation,
        };

        lock_recover(&self.inner.task_meta, "task metadata").insert(task_id, meta);
        self.inner.in_flight.fetch_add(1, Ordering::Relaxed);

        #[cfg(not(target_arch = "wasm32"))]
        self.spawn_native(task_id, task_kind, payload, task_key, generation);

        #[cfg(target_arch = "wasm32")]
        self.spawn_web(task_id, task_kind, payload, task_key, generation);

        TaskHandle { id: task_id }
    }

    fn is_stale(&self, result: &TaskResult) -> bool {
        if self.policy() != TaskPolicy::LatestWins {
            return false;
        }
        let (Some(task_key), Some(generation)) = (&result.task_key, result.generation) else {
            return false;
        };
        let latest = lock_recover(&self.inner.latest_generation, "task latest_generation");
        latest
            .get(task_key)
            .copied()
            .map(|latest_generation| latest_generation > generation)
            .unwrap_or(false)
    }

    fn notify_handlers(&self, result: TaskResult) {
        RESULT_HANDLERS.with(|handlers| {
            if let Some(list) = handlers.borrow_mut().get_mut(&self.id) {
                for handler in list.iter_mut() {
                    handler(result.clone());
                }
            }
        });
    }

    fn try_finish_task(&self, task_id: TaskId) -> bool {
        let removed = lock_recover(&self.inner.task_meta, "task metadata")
            .remove(&task_id)
            .is_some();

        if !removed {
            return false;
        }

        self.inner.in_flight.fetch_sub(1, Ordering::Relaxed);

        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(native) = self.inner.native.as_ref() {
                lock_recover(&native.handles, "task handles").remove(&task_id);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(web) = self.inner.web.as_ref() {
                lock_recover(web, "web task runtime")
                    .task_workers
                    .remove(&task_id);
            }
        }

        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_native(
        &self,
        task_id: TaskId,
        task_kind: String,
        payload: TaskPayload,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
    ) {
        let Some(native) = self.inner.native.as_ref() else {
            if self.try_finish_task(task_id) {
                let _ = self.inner.completion_tx.send(TaskResult {
                    task_id,
                    task_kind,
                    task_key,
                    generation,
                    status: TaskStatus::Error(
                        "task runtime unavailable: native runtime missing".to_owned(),
                    ),
                });
            }
            return;
        };

        let runtime = self.clone();
        let completion_tx = self.inner.completion_tx.clone();
        let task_kind_for_result = task_kind.clone();

        let handle = native.runtime.spawn(async move {
            let status = execute_native_task(&task_kind, payload).await;
            if !runtime.try_finish_task(task_id) {
                return;
            }

            let result = TaskResult {
                task_id,
                task_kind: task_kind_for_result,
                task_key,
                generation,
                status: match status {
                    Ok(payload) => TaskStatus::Success(payload),
                    Err(err) => TaskStatus::Error(err),
                },
            };
            let _ = completion_tx.send(result);
        });

        lock_recover(&native.handles, "task handles").insert(task_id, handle);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_worker_script_url(&self, worker_script_url: impl Into<String>) {
        let worker_script_url = worker_script_url.into();
        let Some(web) = self.inner.web.as_ref() else {
            return;
        };
        let mut web = lock_recover(web, "web task runtime");
        if web.worker_script_url == worker_script_url {
            return;
        }
        for slot in web.workers.drain(..) {
            slot.worker.terminate();
        }
        web.worker_script_url = worker_script_url;
        web.next_worker = 0;
        web.next_worker_token = 1;
        web.task_workers.clear();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_worker_script_url(&self, _worker_script_url: impl Into<String>) {}

    #[cfg(target_arch = "wasm32")]
    fn spawn_web(
        &self,
        task_id: TaskId,
        task_kind: String,
        payload: TaskPayload,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
    ) {
        if let Err(err) = self.ensure_workers() {
            if self.try_finish_task(task_id) {
                let _ = self.inner.completion_tx.send(TaskResult {
                    task_id,
                    task_kind,
                    task_key,
                    generation,
                    status: TaskStatus::Error(err),
                });
            }
            return;
        }

        let request = WorkerRequest::Run {
            task_id,
            task_kind: task_kind.clone(),
            task_key: task_key.clone(),
            generation,
            payload_json: payload.to_string(),
        };

        let (worker_index, worker) = {
            let Some(web_mutex) = self.inner.web.as_ref() else {
                if self.try_finish_task(task_id) {
                    let _ = self.inner.completion_tx.send(TaskResult {
                        task_id,
                        task_kind,
                        task_key,
                        generation,
                        status: TaskStatus::Error(
                            "task runtime unavailable: web worker pool missing".to_owned(),
                        ),
                    });
                }
                return;
            };
            let mut web = lock_recover(web_mutex, "web task runtime");
            if web.workers.is_empty() {
                if self.try_finish_task(task_id) {
                    let _ = self.inner.completion_tx.send(TaskResult {
                        task_id,
                        task_kind,
                        task_key,
                        generation,
                        status: TaskStatus::Error("web worker pool is empty".to_owned()),
                    });
                }
                return;
            }
            let worker_index = web.next_worker % web.workers.len();
            web.next_worker = (web.next_worker + 1) % web.workers.len();
            let worker_token = web.workers[worker_index].token;
            web.task_workers.insert(task_id, worker_token);
            (worker_index, web.workers[worker_index].worker.clone())
        };

        let request_js = match serde_wasm_bindgen::to_value(&request) {
            Ok(value) => value,
            Err(err) => {
                if let Some(web) = self.inner.web.as_ref() {
                    lock_recover(web, "web task runtime")
                        .task_workers
                        .remove(&task_id);
                }
                if self.try_finish_task(task_id) {
                    let _ = self.inner.completion_tx.send(TaskResult {
                        task_id,
                        task_kind,
                        task_key,
                        generation,
                        status: TaskStatus::Error(format!(
                            "failed to encode worker request: {err}"
                        )),
                    });
                }
                return;
            }
        };

        if let Err(err) = worker.post_message(&request_js) {
            if let Some(web) = self.inner.web.as_ref() {
                lock_recover(web, "web task runtime")
                    .task_workers
                    .remove(&task_id);
            }
            if self.try_finish_task(task_id) {
                let _ = self.inner.completion_tx.send(TaskResult {
                    task_id,
                    task_kind,
                    task_key,
                    generation,
                    status: TaskStatus::Error(format!(
                        "failed to post task to worker #{worker_index}: {:?}",
                        err
                    )),
                });
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn ensure_workers(&self) -> Result<(), String> {
        let worker_count = default_web_workers();
        let Some(web_mutex) = self.inner.web.as_ref() else {
            return Err(self
                .inner
                .disabled_reason
                .clone()
                .unwrap_or_else(|| "web task runtime unavailable".to_owned()));
        };
        let mut web = lock_recover(web_mutex, "web task runtime");
        if !web.workers.is_empty() {
            return Ok(());
        }

        let worker_script_url = web.worker_script_url.clone();
        while web.workers.len() < worker_count {
            let worker_token = web.next_worker_token;
            web.next_worker_token += 1;
            match create_worker_slot(worker_script_url.as_str(), self.clone(), worker_token) {
                Ok(slot) => web.workers.push(slot),
                Err(err) => {
                    log::warn!("failed to initialize worker: {err}");
                    break;
                }
            }
        }

        if web.workers.is_empty() {
            return Err(format!(
                "unable to start web worker pool from '{}'",
                worker_script_url
            ));
        }
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn send_web_cancel(&self, task_id: TaskId) {
        let maybe_worker = {
            let Some(web_mutex) = self.inner.web.as_ref() else {
                return;
            };
            let web = lock_recover(web_mutex, "web task runtime");
            let Some(worker_token) = web.task_workers.get(&task_id).copied() else {
                return;
            };
            web.workers
                .iter()
                .find(|slot| slot.token == worker_token)
                .map(|slot| slot.worker.clone())
        };

        let Some(worker) = maybe_worker else {
            return;
        };

        let cancel_message = WorkerRequest::Cancel { task_id };
        if let Ok(js_value) = serde_wasm_bindgen::to_value(&cancel_message) {
            let _ = worker.post_message(&js_value);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn handle_web_worker_error(&self, worker_token: u64, message: String) {
        let affected_tasks = {
            let Some(web_mutex) = self.inner.web.as_ref() else {
                return;
            };
            let mut web = lock_recover(web_mutex, "web task runtime");
            let Some(index) = web.workers.iter().position(|slot| slot.token == worker_token) else {
                return;
            };
            let slot = web.workers.remove(index);
            slot.worker.terminate();
            if web.workers.is_empty() {
                web.next_worker = 0;
            } else if web.next_worker > index {
                web.next_worker -= 1;
                if web.next_worker >= web.workers.len() {
                    web.next_worker = 0;
                }
            } else if web.next_worker >= web.workers.len() {
                web.next_worker = 0;
            }
            web.task_workers
                .iter()
                .filter_map(|(task_id, token)| (*token == worker_token).then_some(*task_id))
                .collect::<Vec<_>>()
        };

        for task_id in affected_tasks {
            let meta = {
                let task_meta = lock_recover(&self.inner.task_meta, "task metadata");
                task_meta.get(&task_id).cloned()
            };
            let Some(meta) = meta else {
                continue;
            };
            if !self.try_finish_task(task_id) {
                continue;
            }
            let _ = self.inner.completion_tx.send(TaskResult {
                task_id,
                task_kind: meta.task_kind,
                task_key: meta.task_key,
                generation: meta.generation,
                status: TaskStatus::Error(message.clone()),
            });
        }
    }
}

struct Inner {
    policy: Mutex<TaskPolicy>,
    next_task_id: AtomicU64,
    in_flight: AtomicUsize,
    latest_generation: Mutex<HashMap<TaskKey, Generation>>,
    task_meta: Mutex<HashMap<TaskId, TaskMeta>>,
    completion_tx: mpsc::Sender<TaskResult>,
    completion_rx: Mutex<mpsc::Receiver<TaskResult>>,
    disabled_reason: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    native: Option<NativeState>,
    #[cfg(target_arch = "wasm32")]
    web: Option<Mutex<WebState>>,
}

#[derive(Clone)]
struct TaskMeta {
    task_kind: String,
    task_key: Option<TaskKey>,
    generation: Option<Generation>,
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeState {
    runtime: tokio::runtime::Runtime,
    handles: Mutex<HashMap<TaskId, JoinHandle<()>>>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
static FORCE_NATIVE_RUNTIME_INIT_FAILURE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(not(target_arch = "wasm32"))]
fn build_native_state() -> Result<NativeState, TaskRuntimeInitError> {
    #[cfg(all(test, not(target_arch = "wasm32")))]
    if FORCE_NATIVE_RUNTIME_INIT_FAILURE.load(Ordering::Relaxed) {
        return Err(TaskRuntimeInitError::NativeRuntime(
            "forced runtime initialization failure".to_owned(),
        ));
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .worker_threads(default_native_workers())
        .build()
        .map_err(|err| TaskRuntimeInitError::NativeRuntime(err.to_string()))?;

    Ok(NativeState {
        runtime,
        handles: Mutex::new(HashMap::new()),
    })
}

#[cfg(target_arch = "wasm32")]
struct WebState {
    worker_script_url: String,
    workers: Vec<WebWorkerSlot>,
    next_worker: usize,
    next_worker_token: u64,
    task_workers: HashMap<TaskId, u64>,
}

#[cfg(target_arch = "wasm32")]
struct WebWorkerSlot {
    token: u64,
    worker: web_sys::Worker,
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

#[cfg(not(target_arch = "wasm32"))]
fn default_native_workers() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get().saturating_sub(1).max(1))
        .unwrap_or(1)
}

#[cfg(target_arch = "wasm32")]
fn default_web_workers() -> usize {
    let hardware_concurrency = web_sys::window()
        .map(|window| window.navigator().hardware_concurrency())
        .unwrap_or(1.0);
    let rounded = hardware_concurrency.round();
    let concurrency = if rounded.is_finite() && rounded >= 1.0 {
        rounded as usize
    } else {
        1
    };
    concurrency.saturating_sub(1).max(1)
}

#[cfg(not(target_arch = "wasm32"))]
async fn execute_native_task(task_kind: &str, payload: TaskPayload) -> Result<TaskPayload, String> {
    match task_kind {
        "echo" => Ok(payload),
        "sleep_echo" => {
            let millis = payload.get("millis").and_then(Value::as_u64).unwrap_or(0);
            let response = payload
                .get("data")
                .cloned()
                .unwrap_or_else(|| payload.clone());
            tokio::time::sleep(Duration::from_millis(millis)).await;
            Ok(response)
        }
        "analyze_text" => {
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| "analyze_text expects payload.text".to_owned())?
                .to_owned();
            tokio::task::spawn_blocking(move || analyze_text_payload(&text))
                .await
                .map_err(|err| format!("analyze_text task join error: {err}"))
        }
        _ => Err(format!("unknown task kind: {task_kind}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn analyze_text_payload(text: &str) -> TaskPayload {
    let trimmed = text.trim();
    let word_count = if trimmed.is_empty() {
        0
    } else {
        trimmed.split_whitespace().count()
    };
    let line_count = if text.is_empty() {
        0
    } else {
        text.lines().count()
    };
    let char_count = text.chars().count();
    let preview: String = text.chars().take(48).collect();

    json!({
        "word_count": word_count,
        "line_count": line_count,
        "char_count": char_count,
        "preview": preview,
    })
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkerRequest {
    Run {
        task_id: TaskId,
        task_kind: String,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
        payload_json: String,
    },
    Cancel {
        task_id: TaskId,
    },
}

#[cfg(target_arch = "wasm32")]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WorkerResponse {
    Done {
        task_id: TaskId,
        task_kind: String,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
        payload_json: String,
    },
    Error {
        task_id: TaskId,
        task_kind: String,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
        message: String,
    },
    Canceled {
        task_id: TaskId,
        task_kind: String,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
    },
}

#[cfg(target_arch = "wasm32")]
fn create_worker_slot(
    worker_script_url: &str,
    runtime: TaskRuntime,
    worker_token: u64,
) -> Result<WebWorkerSlot, String> {
    let worker = web_sys::Worker::new(worker_script_url)
        .map_err(|err| format!("failed to create worker '{worker_script_url}': {:?}", err))?;

    let completion_tx = runtime.inner.completion_tx.clone();
    let runtime_for_message = runtime.clone();
    let on_message = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let response: WorkerResponse = match serde_wasm_bindgen::from_value(event.data()) {
            Ok(response) => response,
            Err(err) => {
                runtime_for_message.handle_web_worker_error(
                    worker_token,
                    format!("failed to decode worker response: {err}"),
                );
                return;
            }
        };

        let (task_id, task_result) = match response {
            WorkerResponse::Done {
                task_id,
                task_kind,
                task_key,
                generation,
                payload_json,
            } => (
                task_id,
                TaskResult {
                    task_id,
                    task_kind,
                    task_key,
                    generation,
                    status: TaskStatus::Success(
                        serde_json::from_str::<TaskPayload>(&payload_json).unwrap_or(Value::Null),
                    ),
                },
            ),
            WorkerResponse::Error {
                task_id,
                task_kind,
                task_key,
                generation,
                message,
            } => (
                task_id,
                TaskResult {
                    task_id,
                    task_kind,
                    task_key,
                    generation,
                    status: TaskStatus::Error(message),
                },
            ),
            WorkerResponse::Canceled {
                task_id,
                task_kind,
                task_key,
                generation,
            } => (
                task_id,
                TaskResult {
                    task_id,
                    task_kind,
                    task_key,
                    generation,
                    status: TaskStatus::Canceled,
                },
            ),
        };

        if !runtime_for_message.try_finish_task(task_id) {
            return;
        }

        let _ = completion_tx.send(task_result);
    }) as Box<dyn FnMut(_)>);

    let runtime_for_error = runtime.clone();
    let on_error = Closure::wrap(Box::new(move |event: web_sys::ErrorEvent| {
        let message = format!(
            "worker runtime error at {}:{}:{}: {}",
            event.filename(),
            event.lineno(),
            event.colno(),
            event.message()
        );
        log::error!("{message}");
        runtime_for_error.handle_web_worker_error(worker_token, message);
    }) as Box<dyn FnMut(_)>);

    worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    Ok(WebWorkerSlot {
        token: worker_token,
        worker,
        _on_message: on_message,
        _on_error: on_error,
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use std::{sync::Mutex, thread, time::Duration as StdDuration};

    static TASK_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn drain_for(runtime: &TaskRuntime, timeout_ms: u64) -> Vec<TaskResult> {
        let mut results = Vec::new();
        let mut elapsed = 0;
        while elapsed <= timeout_ms {
            runtime.drain_completed(|result| results.push(result));
            if !runtime.has_in_flight() {
                runtime.drain_completed(|result| results.push(result));
                break;
            }
            thread::sleep(StdDuration::from_millis(10));
            elapsed += 10;
        }
        results
    }

    #[test]
    fn latest_wins_drops_stale_generations() {
        let _guard = TASK_TEST_LOCK.lock().unwrap();
        let runtime = TaskRuntime::new();
        runtime.set_current();

        runtime.spawn_keyed(
            "todos.analyze",
            1,
            "sleep_echo",
            json!({ "millis": 60, "data": { "tag": "old" } }),
        );
        runtime.spawn_keyed(
            "todos.analyze",
            2,
            "sleep_echo",
            json!({ "millis": 5, "data": { "tag": "new" } }),
        );

        let results = drain_for(&runtime, 500);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].status, TaskStatus::Success(_)));
        if let TaskStatus::Success(payload) = &results[0].status {
            assert_eq!(payload.get("tag").and_then(Value::as_str), Some("new"));
        }
    }

    #[test]
    fn cancel_emits_canceled_result() {
        let _guard = TASK_TEST_LOCK.lock().unwrap();
        let runtime = TaskRuntime::new();
        runtime.set_current();

        let handle = runtime.spawn("sleep_echo", json!({ "millis": 200, "data": "value" }));
        assert!(runtime.cancel(handle.id()));

        let results = drain_for(&runtime, 300);
        assert!(results.iter().any(|result| result.task_id == handle.id()));
        assert!(results
            .iter()
            .any(|result| matches!(result.status, TaskStatus::Canceled)));
    }

    #[test]
    fn task_errors_are_reported() {
        let _guard = TASK_TEST_LOCK.lock().unwrap();
        let runtime = TaskRuntime::new();
        runtime.set_current();

        runtime.spawn("unknown_kind", json!({}));
        let results = drain_for(&runtime, 300);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].status, TaskStatus::Error(_)));
    }

    #[test]
    fn current_or_default_stays_non_panicking_when_runtime_init_fails() {
        let _guard = TASK_TEST_LOCK.lock().unwrap();
        FORCE_NATIVE_RUNTIME_INIT_FAILURE.store(true, Ordering::Relaxed);
        CURRENT_TASK_RUNTIME.with(|slot| {
            *slot.borrow_mut() = None;
        });

        let runtime = TaskRuntime::current_or_default();
        runtime.spawn("echo", json!({ "value": 1 }));
        let results = drain_for(&runtime, 50);

        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].status, TaskStatus::Error(_)));
        assert!(!runtime.has_in_flight());

        CURRENT_TASK_RUNTIME.with(|slot| {
            *slot.borrow_mut() = None;
        });
        FORCE_NATIVE_RUNTIME_INIT_FAILURE.store(false, Ordering::Relaxed);
    }
}
