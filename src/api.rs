use std::collections::HashMap;
use std::error::Error;

use log::{error, info, trace};
use url::Url;

use crate::process_variables::{parse_process_instance_variables, ProcessInstanceVariable};
use crate::structures::{ConfigParams, ServiceTask};

pub async fn get_open_service_tasks(config: &ConfigParams) -> Result<Vec<ServiceTask>, Box<dyn Error>> {
    let mut service_tasks_endpoint = config.url().clone();
    service_tasks_endpoint.set_path("engine-rest/external-task");
    info!("Fetch data at {}", service_tasks_endpoint);

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
                service_tasks_endpoint,
                err
            );
            Err(err.into())
        }
    }
}

pub fn build_authenticated_request(
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

pub fn build_authenticated_post(
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

pub async fn lock_external_task(
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

pub async fn get_process_instance_variables(
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompleteRequest<'a> {
    worker_id: &'a str,
    variables: crate::types::OutputVariables,
}

pub async fn complete_external_task(
    config: &ConfigParams,
    external_task_id: &str,
    variables: crate::types::OutputVariables,
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
