#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::TaskExecutorBackend;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::TaskExecutorBackend;
