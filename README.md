# Operaton Task Worker for Rust

This project implements a basic [Operaton](https://operaton.org) Task worker for performing execution of Service 
Tasks in Operaton BPMN processes using Rust. It periodically polls the Operaton Task Service for new tasks, handles the execution 
of the tasks and updates the Operaton Task Service with the results.

## Operaton
[Operaton](https://operaton.org) is an open source BPMN engine and a fork of Camunda 7.
It provides an API to pull pending external tasks and return results to the engine, which then updates the task state.
The crate uses the Operaton API to poll for external tasks and execute them via a handler function.

## Compatibility
The crate is tested with Operaton 1.0 and intends to provide a stable abstraction layer for future Operaton versions.
Camunda 7 is not supported, however, at the current state, it should be possible to use the crate with Camunda 7 as well.

## How to use this crate

Running a task worker with this crate is intended to be very easy and involves two steps:
- Implement handler functions for the tasks to be executed.
- Start the task worker with the proper configuration.

A minimal working example of a task worker with one handler function looks like this:

```rust
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

// Fetch a task by the topic id (recommended)
#[task_handler(topic = "grant-approval")]
fn service_task_grant_approval_by_topic(_input: &operaton_task_worker::types::InputVariables) -> Result<operaton_task_worker::types::OutputVariables, Box<dyn std::error::Error>> {
  Ok(std::collections::HashMap::new())
}

// Fetch a task by its task_id
#[task_handler(name = "ServiceTask_Grant_Approval")]
fn service_task_grant_approval_by_task_id(_input: &operaton_task_worker::types::InputVariables) -> Result<operaton_task_worker::types::OutputVariables, Box<dyn std::error::Error>> {
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

```rust
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

### Registering a task handler

Create a function with the `task_handler` attribute and annotate it with the **topic** or the **name** of the task to be handled.
The function must have the following signature:

#### Registering task handler by topic
```rust
#[task_handler(topic = "topic_name")]
fn any_function_name(_input: &operaton_task_worker::types::InputVariables) -> Result<operaton_task_worker::types::OutputVariables, Box<dyn std::error::Error>>
```

#### Registering task handler by task id
```rust
#[task_handler(name = "ServiceTask_ID")]
fn any_function_name(_input: &operaton_task_worker::types::InputVariables) -> Result<operaton_task_worker::types::OutputVariables, Box<dyn std::error::Error>>
```

### Input variables
The input variables are a `HashMap` of `String` to `structures::ProcessInstanceVariable`.
The values are deserialized and are statically typed according to the type of the variable.

`Object` variables serialized as JSON (for example with `objectTypeName = java.util.ArrayList`) are parsed into JSON values.
You can deserialize them directly into your domain types with `as_typed`:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct WishlistItem {
  name: String,
  amount: u32,
}

fn read_wishlist(input: &operaton_task_worker::types::InputVariables) {
  let wishlist: Vec<WishlistItem> = input
    .get("wishlist")
    .expect("missing variable wishlist")
    .as_typed()
    .expect("wishlist is not valid JSON array");

  println!("Loaded {} wishlist entries", wishlist.len());
}
```

Example with nested structs and optional fields:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Address {
  city: String,
  street: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Customer {
  id: String,
  address: Address,
}

#[derive(Debug, Deserialize)]
struct OrderLine {
  sku: String,
  quantity: u32,
}

#[derive(Debug, Deserialize)]
struct OrderPayload {
  customer: Customer,
  lines: Vec<OrderLine>,
  note: Option<String>,
}

fn read_orders(input: &operaton_task_worker::types::InputVariables) {
  let orders: Vec<OrderPayload> = input
    .get("orders")
    .expect("missing variable orders")
    .as_typed()
    .expect("orders is not a valid serialized JSON array");

  for order in orders {
    println!("Customer {} from {}", order.customer.id, order.customer.address.city);
  }
}
```

If you want to access an `Object` variable as a plain string (for example to forward the serialized payload), use `as_object_string()`:

```rust
fn forward_object_payload(input: &operaton_task_worker::types::InputVariables) {
  let payload: String = input
    .get("orders")
    .expect("missing variable orders")
    .as_object_string()
    .expect("orders is not an Object variable");

  println!("Forwarding payload: {}", payload);
}
```

### Returning successful executions
- Return `Ok(HashMap::new())` to indicate that the task was executed successfully.
- Return `Ok(...)` with a non-empty output variable map to indicate that the task was executed successfully and that the output variables should be updated.

To return JSON output variables you have two options:

- `out_json(...)` sends a typed variable with `type = Json`.
- `out_json_object(...)` sends a typed variable with `type = Object` and JSON serialization metadata (often more compatible with Java-side object handling).

```rust
use operaton_task_worker::types::{out_json, out_json_object, OutputVariables};

fn build_output() -> OutputVariables {
  let mut result = std::collections::HashMap::new();

  let output = serde_json::json!({
    "status": "ok",
    "items": [1, 2, 3]
  });

  // Json typed value
  result.insert("json_out".to_string(), out_json(&output));

  // Object typed value with serializationDataFormat=application/json.
  // Arrays are emitted as objectTypeName=java.util.ArrayList.
  result.insert("json_object_out".to_string(), out_json_object(&output));

  result
}
```

Troubleshooting (`ENGINE-02041 Class 'java.lang.String' doesn't implement '...DelegateVariableMapping'`):
- This error is usually caused by BPMN model configuration (`delegateVariableMapping`) resolving to a String at runtime, not by JSON serialization alone.
- If this appears when writing a JSON output variable, verify your Call Activity variable mapping setup and consider writing JSON as `Object` via `out_json_object(...)`.

### Returning errors from a handler
- For a BPMN Business Error (Camunda 7/Operaton), return `Err(Box::new(BpmnError::new(code, message)))`.
  The worker will call `/external-task/{id}/bpmnError`.
- For technical failures, return any other error; the worker calls `/external-task/{id}/failure` with `retries=0`.

## Questions and contributions

Feel free to open an issue or a pull request.