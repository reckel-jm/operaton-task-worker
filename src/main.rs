/// Operaton Task Worker is a program that periodically fetches open service tasks from the
/// Operaton instance at Energy Lab and then will try to solve them if any function or sub program
/// is available.

mod structures;
mod process_variables;

use std::collections::HashMap;
use std::error::Error;
use config::Config;
use structures::ConfigParams;

use log::{debug, error, warn, log_enabled, info, Level, trace};
use url::Url;
use crate::process_variables::{ProcessInstanceVariable, parse_process_instance_variables};
use crate::structures::ServiceTask;

// ===== Types for external task function handling and completion payloads =====
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutVariable {
    #[serde(rename = "value")]
    pub value: serde_json::Value,
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(rename = "valueInfo")]
    pub value_info: std::collections::HashMap<String, serde_json::Value>,
}

pub type InputVariables = std::collections::HashMap<String, crate::process_variables::ProcessInstanceVariable>;
pub type OutputVariables = std::collections::HashMap<String, OutVariable>;
pub type ExternalTaskFn = fn(&InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>>;

fn out_string(value: impl Into<String>) -> OutVariable {
    OutVariable {
        value: serde_json::Value::String(value.into()),
        typ: "String".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

fn out_bool(value: bool) -> OutVariable {
    OutVariable {
        value: serde_json::Value::Bool(value),
        typ: "Boolean".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

fn out_json(value: &serde_json::Value) -> OutVariable {
    let mut value_info = std::collections::HashMap::new();
    value_info.insert(
        "serializationDataFormat".to_string(),
        serde_json::Value::String("application/json".to_string()),
    );
    OutVariable {
        // Camunda 7 expects JSON to be provided as a serialized string with serializationDataFormat
        value: serde_json::Value::String(value.to_string()),
        typ: "Json".to_string(),
        value_info,
    }
}

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

                for service_task in service_tasks {
                    // Try to lock the specific external task and read its input variables
                    if let Err(err) = lock_external_task(&config, service_task.id(), 60_000).await {
                        warn!("Could not lock task {}: {:#?}", service_task.id(), err);
                        continue;
                    }

                    let input_vars: HashMap<String, ProcessInstanceVariable> = get_process_instance_variables(&config, service_task.process_instance_id()).await.unwrap_or_else(|err| {
                        error!("Error while fetching external task variables: {:#?}", err);
                        HashMap::new()
                    });
                    trace!("External task variables for {} => {:#?}", service_task.id(), input_vars);

                    match map_service_task_to_function(&service_task) {
                        Some(function) => {
                            debug!("Executing function for Service Task: {:#?}", service_task);
                            match function(&input_vars) {
                                Ok(output_vars) => {
                                    if let Err(err) = complete_external_task(&config, service_task.id(), output_vars).await {
                                        error!("Could not complete external task {}: {:#?}", service_task.id(), err);
                                    } else {
                                        info!("Completed external task {}", service_task.id());
                                    }
                                }
                                Err(err) => {
                                    error!("Execution of function for Service Task {} failed: {:#?}", service_task.id(), err);
                                }
                            }
                        },
                        None => {
                            warn!("No function found for Service Task: {:#?}. SKIP.", service_task.activity_id());
                        }
                    }
                };
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

fn build_authenticated_request(
    client: &reqwest::Client,
    url: Url,
    username: &str,
    password: &str,
) -> reqwest::RequestBuilder {
    let mut request = client.get(url);

    if !username.is_empty() {
        request = request.basic_auth(username, Some(password));
        trace!("Using HTTP Basic authentication");
    } else {
        trace!("No HTTP authentication configured (empty username)");
    }

    request
}

fn build_authenticated_post(
    client: &reqwest::Client,
    url: Url,
    username: &str,
    password: &str,
) -> reqwest::RequestBuilder {
    let mut request = client.post(url);

    if !username.is_empty() {
        request = request.basic_auth(username, Some(password));
        trace!("Using HTTP Basic authentication");
    } else {
        trace!("No HTTP authentication configured (empty username)");
    }

    request
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LockRequest<'a> {
    worker_id: &'a str,
    lock_duration: u64,
}

async fn lock_external_task(
    config: &ConfigParams,
    external_task_id: &str,
    lock_duration_ms: u64,
) -> Result<(), Box<dyn Error>> {
    let mut endpoint = config.url().clone();
    let path_string = format!(
        "engine-rest/external-task/{}/lock",
        external_task_id
    );
    endpoint.set_path(path_string.as_str());
    info!("Lock external task at {}", endpoint);

    let client = reqwest::Client::new();
    let request = build_authenticated_post(
        &client,
        endpoint.clone(),
        config.username(),
        config.password(),
    )
    .json(&LockRequest { worker_id: config.id(), lock_duration: lock_duration_ms });

    let response = request.send().await.map_err(|err| {
        error!(
            "Error while calling API endpoint '{}': {:#?}",
            endpoint, err
        );
        err
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
        error!("Lock request failed: status={} body={} ", status, body);
        return Err(format!("Lock failed with status {status}").into());
    }

    trace!("Task '{}' locked for {} ms", external_task_id, lock_duration_ms);
    Ok(())
}

async fn get_process_instance_variables(
    config: &ConfigParams,
    process_instance_id: &str,
) -> Result<HashMap<String, ProcessInstanceVariable>, Box<dyn Error>> {
    let mut endpoint = config.url().clone();
    let path_string = "engine-rest/variable-instance";

    endpoint.set_path(path_string);
    endpoint.set_query(Some(format!("processInstanceIdIn={}", process_instance_id).as_str()));

    info!("Fetch external task variables at {}", endpoint);

    let client = reqwest::Client::new();
    let request = build_authenticated_request(
        &client,
        endpoint.clone(),
        config.username(),
        config.password(),
    );

    let response = request.send().await.map_err(|err| {
        error!(
            "Error while calling API endpoint '{}': {:#?}",
            endpoint, err
        );
        err
    })?;

    let body = response.text().await.map_err(|err| {
        error!("An error occurred while reading the response body: {:#?}", err);
        err
    })?;

    trace!("Variables raw: {}", body);

    let parsed = parse_process_instance_variables(&body);
    trace!("Parsed variables: {:#?}", parsed);

    Ok(parsed)
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
fn map_service_task_to_function(service_task: &ServiceTask) -> Option<ExternalTaskFn> {
    match service_task.activity_id() {
        // Example mapping: return a demo function for a known activity id
        "example_echo" => Some(example_echo),
        "ServiceTask_GetScannedFiles" => Some(get_scanned_files),
        _ => None,
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompleteRequest<'a> {
    worker_id: &'a str,
    variables: OutputVariables,
}

async fn complete_external_task(
    config: &ConfigParams,
    external_task_id: &str,
    variables: OutputVariables,
) -> Result<(), Box<dyn Error>> {
    let mut endpoint = config.url().clone();
    let path_string = format!(
        "engine-rest/external-task/{}/complete",
        external_task_id
    );
    endpoint.set_path(path_string.as_str());
    info!("Complete external task at {}", endpoint);

    let client = reqwest::Client::new();
    let request = build_authenticated_post(
        &client,
        endpoint.clone(),
        config.username(),
        config.password(),
    )
    .json(&CompleteRequest { worker_id: config.id(), variables });

    let response = request.send().await.map_err(|err| {
        error!(
            "Error while calling API endpoint '{}': {:#?}",
            endpoint, err
        );
        err
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
        error!("Complete request failed: status={} body={} ", status, body);
        return Err(format!("Complete failed with status {status}").into());
    }

    trace!("Task '{}' completed", external_task_id);
    Ok(())
}

fn example_echo(input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("workerResponse".to_string(), out_string("ok"));

    // Return a summary JSON of the input variable names
    let keys: Vec<&String> = input.keys().collect();
    let summary = serde_json::json!({ "keys": keys });
    out.insert("summary".to_string(), out_json(&summary));

    Ok(out)
}

fn get_scanned_files(input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("FILENAMES".to_string(), out_string("TEST"));
    Ok(out)
}
