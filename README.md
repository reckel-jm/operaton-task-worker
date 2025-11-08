# Operaton Task Worker for Data Management Processes at Energy Lab 2.0

This project implements a basic [Operaton](https://operaton.org) Task worker for performing execution of Service 
Tasks in Operaton BPMN processes. It periodically polls the Operaton Task Service for new tasks, handles the execution 
of the tasks and updates the Operaton Task Service with the results.

## Module overview
- `src/main.rs` — orchestrates the poll → lock → fetch variables → execute handler → complete/fail flow.
- `src/settings.rs` — loads config from env; see variables below.
- `src/structures.rs` — data structures for config and service tasks from the engine.
- `src/process_variables/` — parsing of process instance variables and accessors.
- `src/types.rs` — handler types, `OutVariable` helpers, and `BpmnError` type for business errors.
- `src/api.rs` — HTTP client helpers and REST calls (get tasks, lock, get variables, complete, failure, bpmnError).
- `src/registry.rs` — handler registry powered by the `inventory` crate.
- `src/handlers/` — your external task handler functions (examples in `builtin.rs`).
- `src/lib.rs` — proc-macro crate that provides `#[task_handler(name = "...")]` for auto registration.

## Writing a handler
Annotate a function with the attribute macro to auto-register it by `activityId` or `topicName`:

```rust
use crate::types::{InputVariables, OutputVariables, out_string};

#[operaton_task_worker_macros::task_handler(name = "ServiceTask_get_filenames")]
pub fn get_filenames(input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out = OutputVariables::new();
    // Access inputs via helpers
    if let Some(var) = input.get("flag") {
        if var.as_bool() == Some(true) {
            out.insert("result".into(), out_string("ok"));
        }
    }
    Ok(out)
}
```

Available output helpers in `types.rs`: `out_string`, `out_bool`, `out_integer`, `out_long`, `out_double`, `out_json`.
Input helpers in `process_variables`: `as_bool()`, `as_str()`, `as_json()` on `ProcessInstanceVariable`.

### Reporting errors from a handler
- For a BPMN Business Error (Camunda 7/Operaton), return `Err(Box::new(BpmnError::new(code, message)))`.
  The worker will call `/external-task/{id}/bpmnError`.
- For technical failures, return any other error; the worker calls `/external-task/{id}/failure` with `retries=0`.

## Environment Variables

Operaton Task Worker uses the following environment variables:

- `OPERATON_TASK_WORKER_URL` - URL of the Operaton Task Service
- `OPERATON_TASK_WORKER_USERNAME` - Username for the Operaton Task Service (leave empty for anonymous access)
- `OPERATON_TASK_WORKER_PASSWORD` - Password for the Operaton Task Service (leave empty for anonymous access)
- `OPERATON_TASK_WORKER_POLL_INTERVAL` - Interval in milliseconds for polling the Operaton Task Service for new tasks
- `RUST_LOG` - Logging level for the application, e.g. `info,operaton_task_worker=debug`

## Development Setup
- Install Rust and Cargo
- Run `cargo build`
- Run `cargo run`
- Run tests with `cargo test`