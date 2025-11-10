/*
# Operaton Task Worker: A Rust library for external task polling and execution from Operaton

This crate provides functionality to poll external tasks from the Operaton BPMN engine and execute them via a handler function.
It is intended for implementing external task handlers and the BPMN business logic in Rust.

## Operaton
[Operaton](https://operaton.org) is an open source BPMN engine and a fork of Camunda 7.
It provides an API to pull pending external tasks and return results to the engine, which then updates the task state.
The crate uses the Operaton API to poll for external tasks and execute them via a handler function.

## Compatibility
The crate is tested with Operaton 1.0 and intends to provide a stable abstraction layer for future Operaton versions.
Camunda 7 is not supported, however, at the current state, it should be possible to use the crate with Camunda 7.

## How to use

 */

mod polling;
pub mod structures;
pub mod process_variables;
pub mod types;
mod api;
pub mod registry;
pub mod settings;

pub use inventory;
pub use operaton_task_worker_macros::task_handler;

use crate::structures::ConfigParams;

/// Start the polling loop asynchronously. Call this inside a Tokio runtime.
pub async fn poll(config: ConfigParams) {
    polling::start_polling_loop(config).await;
}

/// Convenience: start the polling loop and block the current thread until it ends (infinite loop).
pub fn poll_blocking(config: ConfigParams) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");
    rt.block_on(async move { polling::start_polling_loop(config).await });
}
