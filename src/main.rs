/// Operaton Task Worker is a program that periodically fetches open service tasks from the
/// Operaton instance at Energy Lab and then will try to solve them if any function or sub program
/// is available.

mod structures;
mod process_variables;

use std::error::Error;
use config::Config;
use structures::ConfigParams;

use log::{debug, error, warn, log_enabled, info, Level, trace};
use crate::process_variables::ProcessInstanceVariable;
use crate::structures::ServiceTask;

/// The prefix for all environment variables used by Operaton Task Worker
///
/// Note: This does not apply for Rust-specific environment variables such as `LOGLEVEL`.
const ENV_PREFIX: &str = "OPERATON_TASK_WORKER";

#[tokio::main]
async fn main() {
    // Get the parameters from the environment variables
    let config = load_config();

    env_logger::init();

    info!("Load Operaton Task Worker with configuration: {:#?}", config);

    if config.username().is_empty() || config.password().is_empty() {
        warn!("No authentication set up. Operaton should be protected by authentication in productive use.");
    }

    trace!("Enter the main loop");

    loop {
        match get_open_service_tasks(&config).await {
            Ok(service_tasks) => {
                info!(
                    "We received {} open external Service Tasks from Operaton.",
                    service_tasks.len()
                );

                service_tasks.iter().for_each(|service_task|
                    match map_service_task_to_function(service_task) {
                        Some(function) => {
                            debug!("Executing function for Service Task: {:#?}", service_task);
                            function().expect("TODO: panic message");
                        },
                        None => {
                            warn!("No function found for Service Task: {:#?}. SKIP.", service_task.business_key());
                        }
                    }
                );
            },
            Err(error) => error!("We were unable to receive and parse any Service Tasks. Error: {:#}", error)
        }

        // Wait for the in `config.poll_interval` milliseconds
        tokio::time::sleep(tokio::time::Duration::from_millis(config.poll_interval() as u64)).await;
    }
}

async fn get_open_service_tasks(config: &ConfigParams) -> Result<Vec<ServiceTask>, Box<dyn Error>> {
    let mut service_tasks_endpoint = config.url().clone();
    service_tasks_endpoint.set_path("engine-rest/external-task");
    info!("Fetch data at {}", service_tasks_endpoint.to_string());

    // Build the request with optional Basic Auth when username is provided
    let client = reqwest::Client::new();
    let mut request = client.get(service_tasks_endpoint.clone());

    if !config.username().is_empty() {
        request = request.basic_auth(config.username().to_string(), Some(config.password().to_string()));
        trace!("Using HTTP Basic authentication");
    } else {
        trace!("No HTTP authentication configured (empty username)");
    }

    match request.send().await {
        Ok(response) => {
            match response.json().await {
                Ok(unwrapped_json) => {
                    let service_tasks: Vec<ServiceTask> = unwrapped_json;
                    trace!("Parsed: {:#?}", service_tasks);
                    Ok(service_tasks)
                },
                Err(err) => {
                    error!("An error occurred while parsing the JSON: {:#?}", err);
                    Err(err.into())
                }
            }
        },
        Err(err) => {
            error!(
                "Error while calling API endpoint '{}': {:#?}",
                service_tasks_endpoint.to_string(),
                err
            );
            Err(err.into())
        }
    }
}

/// Loads the configuration into a [ConfigParams] struct. The function may panic, but it should not
/// happen because [ConfigParams] provides default values for all configured entries.
fn load_config() -> ConfigParams {
    let settings = Config::builder()
        .add_source(config::Environment::with_prefix(ENV_PREFIX))
        .build()
        .unwrap();

    settings.try_deserialize().unwrap()
}

/// Maps the Service Task to an executable function.
fn map_service_task_to_function(service_task: &ServiceTask) -> Option<fn() -> Result<Vec<ProcessInstanceVariable>, Box<dyn Error>>> {
    None
}