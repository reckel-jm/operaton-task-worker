/// Operaton Task Worker is a program that periodically fetches open service tasks from the
/// Operaton instance at Energy Lab and then will try to solve them if any function or sub program
/// is available.

mod structures;
mod process_variables;

use std::error::Error;
use config::Config;
use structures::ConfigParams;

use log::{debug, error, warn, log_enabled, info, Level, trace};
use crate::structures::ServiceTask;

#[tokio::main]
async fn main() {
    // Get the parameters from the environment variables
    let config = load_config();

    env_logger::init();

    info!("Load Operaton Task Worker with configuration: {:#?}", config);

    if config.username().is_empty() || config.password().is_empty() {
        warn!("No authentication set up. Operaton should be protected by authentication in productive use.");
    }

    info!("Enter the main loop");

    loop {
        info!("Test");
        match get_open_service_tasks(config).await {
            Ok(service_tasks) => {
                trace!(
                    "We received {} open Service Tasks from Operaton.",
                    service_tasks.len()
                )
            },
            Err(error) => error!("We were unable to receive and parse any Service Tasks")
        }
        break;
    }
}

async fn get_open_service_tasks(config: ConfigParams) -> Result<Vec<ServiceTask>, Box<dyn Error>> {
    let mut service_tasks_endpoint = config.url().clone();
    service_tasks_endpoint.set_path("engine-rest/external-task");
    info!("Fetch data at {}", service_tasks_endpoint.to_string());

    match reqwest::get(service_tasks_endpoint.clone()).await {
        Ok(response) => {
            match response.json().await {
                Ok(unwrapped_json) => {
                    let service_tasks: Vec<ServiceTask> = unwrapped_json;
                    trace!("Parsed: {:#?}", service_tasks);
                    Ok(service_tasks)
                },
                Err(err) => {
                    error!("An error occurred while parsing the JSON: {:#?}",
                    err);
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
        .add_source(config::Environment::with_prefix("OTW"))
        .build()
        .unwrap();

    settings.try_deserialize().unwrap()
}