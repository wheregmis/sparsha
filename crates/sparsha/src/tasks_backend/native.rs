use super::super::{
    Generation, TaskId, TaskKey, TaskPayload, TaskResult, TaskRuntime, TaskRuntimeInitError,
    TaskStatus,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
    time::Duration,
};
use tokio::task::JoinHandle;

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
    runtime: tokio::runtime::Runtime,
    handles: Mutex<HashMap<TaskId, JoinHandle<()>>>,
}

impl TaskExecutorBackend {
    pub(crate) fn try_new() -> Result<Self, TaskRuntimeInitError> {
        #[cfg(all(test, not(target_arch = "wasm32")))]
        if super::super::FORCE_NATIVE_RUNTIME_INIT_FAILURE
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(TaskRuntimeInitError::NativeRuntime(
                "forced runtime initialization failure".to_owned(),
            ));
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .worker_threads(default_native_workers())
            .build()
            .map_err(|err| TaskRuntimeInitError::NativeRuntime(err.to_string()))?;

        Ok(Self {
            runtime,
            handles: Mutex::new(HashMap::new()),
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
        let runtime = runtime.clone();
        let completion_tx = runtime.inner.completion_tx.clone();
        let task_kind_for_result = task_kind.clone();

        let handle = self.runtime.spawn(async move {
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

        lock_recover(&self.handles, "task handles").insert(task_id, handle);
    }

    pub(crate) fn cancel(&self, task_id: TaskId) {
        if let Some(handle) = lock_recover(&self.handles, "task handles").remove(&task_id) {
            handle.abort();
        }
    }

    pub(crate) fn task_finished(&self, task_id: TaskId) {
        lock_recover(&self.handles, "task handles").remove(&task_id);
    }

    pub(crate) fn set_worker_script_url(&self, _worker_script_url: impl Into<String>) {}
}

#[cfg(not(target_arch = "wasm32"))]
fn default_native_workers() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get().saturating_sub(1).max(1))
        .unwrap_or(1)
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
