/*!
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

## How to Use this Crate

Running a task worker with this crate is intended to be very easy and involves two steps:
- Implement handler functions for the tasks to be executed
- Start the task worker with the proper configuration

A minimal working axample of a task worker with one handler function looks like this:

```ignore
use operaton_task_worker::{poll, settings};
use operaton_task_worker_macros::task_handler;

/// The prefix for all environment variables used by Operaton Task Worker
///
/// Note: This does not apply for Rust-specific environment variables such as `LOGLEVEL`.
pub const ENV_PREFIX: &str = "OPERATON_TASK_WORKER";

#[tokio::main]
async fn main() {
  // Get the parameters from the environment variables
  let config = settings::load_config_from_env(ENV_PREFIX);
  poll(config).await;
}

#[task_handler(name = "ServiceTask_Grant_Approval")]
fn service_task_grant_approval(_input: &operaton_task_worker::types::InputVariables) -> Result<operaton_task_worker::types::OutputVariables, Box<dyn std::error::Error>> {
  Ok(std::collections::HashMap::new())
}
```
### Starting the main poll function

The poll function is the main entry point for the task worker. It starts the polling loop and blocks the current thread until it ends (infinite loop).
Use the top level `poll` function for async or the convenience function `poll_blocking` for non async environments.

### Configuring the task worker
The task worker is configured via the `ConfigParams` struct. The struct implementation provides a builder pattern to configure the task worker.
You can also load the configuration from environment variables using the `load_config_from_env` function.


#### Using the environment variables

The following environment variables are used by the task worker--given that the prefix is `OPERATON_TASK_WORKER`:
- `OPERATON_TASK_WORKER_URL` - URL of the Operaton Task Service
- `OPERATON_TASK_WORKER_USERNAME` - Username for the Operaton Task Service (leave empty for anonymous access)
- `OPERATON_TASK_WORKER_PASSWORD` - Password for the Operaton Task Service (leave empty for anonymous access)
- `OPERATON_TASK_WORKER_POLL_INTERVAL` - Interval in milliseconds for polling the Operaton Task Service for new tasks
- `OPERATON_TASK_WORKER_ID` - The task worker id which will be registered with Operaton
- `OPERATON_TASK_WORKER_LOCK_DURATION` - Duration in milliseconds to lock an external task when picked up by this worker (default: 60000)
- `RUST_LOG` - Logging level for the application, e.g. `info,operaton_task_worker=debug`

```ignore
use operaton_task_worker::settings::load_config_from_env;

let config = load_config_from_env("OPERATON_TASK_WORKER"); // or use any other prefix that you like
```
#### Using the builder pattern
```rust
use operaton_task_worker::settings::ConfigParams;
use url::Url;

let config = ConfigParams::default()
    .with_url(Url::parse("http://localhost:8080").unwrap())
    .with_auth("user".to_string(), "pass".to_string())
    .with_poll_interval(1000)
    .with_worker_id("operaton_task_worker".to_string())
    .with_lock_duration(60_000);
```

### Registering a Task Handler

Create a function with the `task_handler` attribute and annotate it with the name of the task to be handled.
The function must have the following signature:

```ignore
#[task_handler(name = "ServiceTask_ID")]
fn any_function_name(_input: &operaton_task_worker::types::InputVariables) -> Result<operaton_task_worker::types::OutputVariables, Box<dyn std::error::Error>>
```

#### Input Variables
The input variables are a `HashMap` of `String` to `structures::ProcessInstanceVariable`.
The values are deserialized and are statically typed according to the type of the variable.

#### Returning Successful Executions
- Return `Ok(HashMap::new())` to indicate that the task was executed successfully.
- Return `Ok(...)` with a non-empty output variable map to indicate that the task was executed successfully and that the output variables should be updated.

#### Returning errors from a handler
- For a BPMN Business Error (Camunda 7/Operaton), return `Err(Box::new(BpmnError::new(code, message)))`.
  The worker will call `/external-task/{id}/bpmnError`.
- For technical failures, return any other error; the worker calls `/external-task/{id}/failure` with `retries=0`.


**/

mod polling;
pub mod structures;
pub mod types;
mod api;
pub mod registry;
pub mod settings;

pub use inventory;
pub use operaton_task_worker_macros::task_handler;

use crate::settings::ConfigParams;

/// Start the polling loop asynchronously. Call this inside a Tokio runtime.
pub async fn poll(config: ConfigParams) {
    polling::start_polling_loop(config).await;
}

/// Convenience: start the polling loop and block the current thread until it ends (infinite loop).
pub fn poll_blocking(config: ConfigParams) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create Tokio runtime");
    rt.block_on(async move { polling::start_polling_loop(config).await });
}
