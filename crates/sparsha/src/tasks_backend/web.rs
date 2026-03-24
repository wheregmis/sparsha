use super::super::{
    Generation, TaskId, TaskKey, TaskPayload, TaskResult, TaskRuntime, TaskRuntimeInitError,
    TaskStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};
use wasm_bindgen::{closure::Closure, JsCast};

fn lock_recover<'a, T>(mutex: &'a Mutex<T>, label: &str) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("recovering from poisoned mutex: {label}");
            poisoned.into_inner()
        }
    }
}

pub(crate) struct TaskExecutorBackend {
    state: Mutex<WebState>,
}

impl TaskExecutorBackend {
    pub(crate) fn try_new() -> Result<Self, TaskRuntimeInitError> {
        Ok(Self {
            state: Mutex::new(WebState {
                worker_script_url: "sparsha-worker.js".to_owned(),
                workers: Vec::new(),
                next_worker: 0,
                next_worker_token: 1,
                task_workers: HashMap::new(),
            }),
        })
    }

    pub(crate) fn spawn(
        &self,
        runtime: &TaskRuntime,
        task_id: TaskId,
        task_kind: String,
        payload: TaskPayload,
        task_key: Option<TaskKey>,
        generation: Option<Generation>,
    ) {
        if let Err(err) = self.ensure_workers(runtime) {
            if runtime.try_finish_task(task_id) {
                let _ = runtime.inner.completion_tx.send(TaskResult {
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
            let mut state = lock_recover(&self.state, "web task runtime");
            if state.workers.is_empty() {
                if runtime.try_finish_task(task_id) {
                    let _ = runtime.inner.completion_tx.send(TaskResult {
                        task_id,
                        task_kind,
                        task_key,
                        generation,
                        status: TaskStatus::Error("web worker pool is empty".to_owned()),
                    });
                }
                return;
            }
            let worker_index = state.next_worker % state.workers.len();
            state.next_worker = (state.next_worker + 1) % state.workers.len();
            let worker_token = state.workers[worker_index].token;
            state.task_workers.insert(task_id, worker_token);
            (worker_index, state.workers[worker_index].worker.clone())
        };

        let request_js = match serde_wasm_bindgen::to_value(&request) {
            Ok(value) => value,
            Err(err) => {
                lock_recover(&self.state, "web task runtime")
                    .task_workers
                    .remove(&task_id);
                if runtime.try_finish_task(task_id) {
                    let _ = runtime.inner.completion_tx.send(TaskResult {
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
            lock_recover(&self.state, "web task runtime")
                .task_workers
                .remove(&task_id);
            if runtime.try_finish_task(task_id) {
                let _ = runtime.inner.completion_tx.send(TaskResult {
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

    pub(crate) fn cancel(&self, task_id: TaskId) {
        let maybe_worker = {
            let state = lock_recover(&self.state, "web task runtime");
            let Some(worker_token) = state.task_workers.get(&task_id).copied() else {
                return;
            };
            state
                .workers
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

    pub(crate) fn task_finished(&self, task_id: TaskId) {
        lock_recover(&self.state, "web task runtime")
            .task_workers
            .remove(&task_id);
    }

    pub(crate) fn set_worker_script_url(&self, worker_script_url: impl Into<String>) {
        let worker_script_url = worker_script_url.into();
        let mut state = lock_recover(&self.state, "web task runtime");
        if state.worker_script_url == worker_script_url {
            return;
        }
        for slot in state.workers.drain(..) {
            slot.worker.terminate();
        }
        state.worker_script_url = worker_script_url;
        state.next_worker = 0;
        state.next_worker_token = 1;
        state.task_workers.clear();
    }

    fn ensure_workers(&self, runtime: &TaskRuntime) -> Result<(), String> {
        let worker_count = default_web_workers();
        let mut state = lock_recover(&self.state, "web task runtime");
        if !state.workers.is_empty() {
            return Ok(());
        }

        let worker_script_url = state.worker_script_url.clone();
        while state.workers.len() < worker_count {
            let worker_token = state.next_worker_token;
            state.next_worker_token += 1;
            match create_worker_slot(worker_script_url.as_str(), runtime.clone(), worker_token) {
                Ok(slot) => state.workers.push(slot),
                Err(err) => {
                    log::warn!("failed to initialize worker: {err}");
                    break;
                }
            }
        }

        if state.workers.is_empty() {
            return Err(format!(
                "unable to start web worker pool from '{}'",
                worker_script_url
            ));
        }
        Ok(())
    }

    fn handle_web_worker_error(&self, runtime: &TaskRuntime, worker_token: u64, message: String) {
        let affected_tasks = {
            let mut state = lock_recover(&self.state, "web task runtime");
            let Some(index) = state
                .workers
                .iter()
                .position(|slot| slot.token == worker_token)
            else {
                return;
            };
            let slot = state.workers.remove(index);
            slot.worker.terminate();
            if state.workers.is_empty() {
                state.next_worker = 0;
            } else if state.next_worker > index {
                state.next_worker -= 1;
                if state.next_worker >= state.workers.len() {
                    state.next_worker = 0;
                }
            } else if state.next_worker >= state.workers.len() {
                state.next_worker = 0;
            }
            state
                .task_workers
                .iter()
                .filter_map(|(task_id, token)| (*token == worker_token).then_some(*task_id))
                .collect::<Vec<_>>()
        };

        for task_id in affected_tasks {
            let meta = {
                let task_meta = lock_recover(&runtime.inner.task_meta, "task metadata");
                task_meta.get(&task_id).cloned()
            };
            let Some(meta) = meta else {
                continue;
            };
            if !runtime.try_finish_task(task_id) {
                continue;
            }
            let _ = runtime.inner.completion_tx.send(TaskResult {
                task_id,
                task_kind: meta.task_kind,
                task_key: meta.task_key,
                generation: meta.generation,
                status: TaskStatus::Error(message.clone()),
            });
        }
    }
}

struct WebState {
    worker_script_url: String,
    workers: Vec<WebWorkerSlot>,
    next_worker: usize,
    next_worker_token: u64,
    task_workers: HashMap<TaskId, u64>,
}

struct WebWorkerSlot {
    token: u64,
    worker: web_sys::Worker,
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

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

fn create_worker_slot(
    worker_script_url: &str,
    runtime: TaskRuntime,
    worker_token: u64,
) -> Result<WebWorkerSlot, String> {
    let worker = web_sys::Worker::new(worker_script_url)
        .map_err(|err| format!("failed to create worker '{worker_script_url}': {:?}", err))?;

    let completion_tx = runtime.inner.completion_tx.clone();
    let runtime_for_message = runtime.clone();
    let runtime_for_backend = runtime.clone();
    let on_message = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let response: WorkerResponse = match serde_wasm_bindgen::from_value(event.data()) {
            Ok(response) => response,
            Err(err) => {
                runtime_for_backend
                    .inner
                    .backend
                    .as_ref()
                    .expect("web backend")
                    .handle_web_worker_error(
                        &runtime_for_backend,
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
        runtime_for_error
            .inner
            .backend
            .as_ref()
            .expect("web backend")
            .handle_web_worker_error(&runtime_for_error, worker_token, message);
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
