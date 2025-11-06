# Operaton Task Worker for the Data Management Processes at Energy 2.0

This project implements a basic [Operaton](https://operaton.org) Task worker for performing execution of Service 
Tasks in Operaton BPMN processes. It periodically polls the Operaton Task Service for new tasks, handles the execution 
of the tasks and updates the Operaton Task Service with the results.

## Environment Variables

Operaton Task Worker uses the following environment variables:

- `OPERATON_TASK_SERVICE_URL` - URL of the Operaton Task Service
- `OPERATON_TASK_SERVICE_USERNAME` - Username for the Operaton Task Service (leave empty for anonymous access)
- `OPERATON_TASK_SERVICE_PASSWORD` - Password for the Operaton Task Service (leave empty for anonymous access)
- `OPERATON_TASK_SERVICE_POLL_INTERVAL` - Interval in milliseconds for polling the Operaton Task Service for new tasks
- `RUST_LOG` - Logging level for the application, e.g. `info,operaton_task_worker=debug`

## Development Setup
- Install Rust and Cargo
- Run `cargo build`
- Run `cargo run`

Implementations of data structures should be tested. Make sure to run tests regulary with `cargo test`.