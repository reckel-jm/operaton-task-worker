use std::collections::HashMap;
use std::error::Error;

use log::{error, info, trace};
use url::Url;

use crate::settings::ConfigParams;
use crate::structures::process_variables::{parse_process_instance_variables, ProcessInstanceVariable};
use crate::types::OutputVariables;
use crate::structures::service_task::ServiceTask;
use crate::registry;

// Put these structs somewhere at the top or middle of your api.rs
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TopicRequest<'a> {
    topic_name: &'a str,
    lock_duration: u64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchAndLockRequest<'a> {
    worker_id: &'a str,
    max_tasks: usize,
    topics: Vec<TopicRequest<'a>>,
}

/// Fetches and atomically locks available tasks for all registered topics.
pub async fn fetch_and_lock_tasks(
    config: &ConfigParams,
    lock_duration_ms: u64,
    max_tasks: usize,
) -> Result<Vec<ServiceTask>, Box<dyn Error>> {
    let topics = registry::topics();

    // If no topics are registered, we cannot use fetchAndLock.
    if topics.is_empty() {
        info!("No topics registered in the handler registry. Skipping fetchAndLock.");
        return Ok(Vec::new());
    }

    let mut endpoint = config.url().clone();
    endpoint.set_path("engine-rest/external-task/fetchAndLock");
    info!("Fetch and lock tasks at {}", endpoint);

    // Map all unique topics from our registry into the request payload
    let topic_requests: Vec<TopicRequest> = topics
        .iter()
        .map(|t| TopicRequest {
            topic_name: t,
            lock_duration: lock_duration_ms,
        })
        .collect();

    let request_body = FetchAndLockRequest {
        worker_id: config.id(),
        max_tasks,
        topics: topic_requests,
    };

    let client = reqwest::Client::new();
    let request = build_authenticated_post(&client, endpoint.clone(), config.username(), config.password())
        .json(&request_body);

    let response = request.send().await.map_err(|err| {
        error!("Error while calling fetchAndLock endpoint '{}': {:#?}", endpoint, err);
        err
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
        error!("fetchAndLock request failed: status={} body={} ", status, body);
        return Err(format!("fetchAndLock failed with status {status}").into());
    }

    // The engine returns an array of already locked tasks
    let service_tasks: Vec<ServiceTask> = response.json().await.map_err(|err| {
        error!("An error occurred while parsing the fetchAndLock JSON: {:#?}", err);
        err
    })?;

    trace!("Successfully fetched and locked {} tasks", service_tasks.len());
    Ok(service_tasks)
}

pub async fn get_open_service_tasks(config: &ConfigParams) -> Result<Vec<ServiceTask>, Box<dyn Error>> {
    // If any handler declares a topic, fetch per-topic and merge results. Otherwise, fetch all.
    let topics = registry::topics();
    let include_unfiltered = registry::has_nontopic_handlers() || topics.is_empty();

    let mut aggregated: Vec<ServiceTask> = Vec::new();

    // Fetch per topic
    for t in &topics {
        let mut endpoint = config.url().clone();
        endpoint.set_path("engine-rest/external-task");
        endpoint.set_query(Some(&format!("topicName={}", t)));
        info!("Fetch data at {}", endpoint);

        let client = reqwest::Client::new();
        let request = build_authenticated_request(&client, endpoint.clone(), config.username(), config.password());

        match request.send().await {
            Ok(response) => {
                match response.json().await {
                    Ok(unwrapped_json) => {
                        let mut service_tasks: Vec<ServiceTask> = unwrapped_json;
                        trace!("Parsed: {:#?}", service_tasks);
                        aggregated.append(&mut service_tasks);
                    },
                    Err(err) => {
                        error!("An error occurred while parsing the JSON: {:#?}", err);
                        return Err(err.into());
                    }
                }
            },
            Err(err) => {
                error!(
                    "Error while calling API endpoint '{}': {:#?}",
                    endpoint,
                    err
                );
                return Err(err.into());
            }
        }
    }

    // If we also need to include unfiltered (handlers without topics), fetch once without topic
    if include_unfiltered {
        let mut endpoint = config.url().clone();
        endpoint.set_path("engine-rest/external-task");
        info!("Fetch data at {}", endpoint);

        let client = reqwest::Client::new();
        let request = build_authenticated_request(&client, endpoint.clone(), config.username(), config.password());

        match request.send().await {
            Ok(response) => {
                match response.json().await {
                    Ok(unwrapped_json) => {
                        let mut service_tasks: Vec<ServiceTask> = unwrapped_json;
                        trace!("Parsed: {:#?}", service_tasks);
                        aggregated.append(&mut service_tasks);
                    },
                    Err(err) => {
                        error!("An error occurred while parsing the JSON: {:#?}", err);
                        return Err(err.into());
                    }
                }
            },
            Err(err) => {
                error!(
                    "Error while calling API endpoint '{}': {:#?}",
                    endpoint,
                    err
                );
                return Err(err.into());
            }
        }
    }

    // Deduplicate by external task id in case overlaps
    aggregated.sort_by(|a,b| a.id().cmp(b.id()));
    aggregated.dedup_by(|a,b| a.id() == b.id());

    Ok(aggregated)
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FailureRequest<'a> {
    worker_id: &'a str,
    error_message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_details: Option<&'a str>,
    retries: i32,
    retry_timeout: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BpmnErrorRequest<'a> {
    worker_id: &'a str,
    error_code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<crate::types::OutputVariables>,
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

pub async fn report_external_task_failure(
    config: &ConfigParams,
    external_task_id: &str,
    error_message: &str,
    error_details: Option<&str>,
    retries: i32,
    retry_timeout_ms: i64,
) -> Result<(), Box<dyn Error>> {
    let mut endpoint = config.url().clone();
    let path_string = format!(
        "engine-rest/external-task/{}/failure",
        external_task_id
    );
    endpoint.set_path(path_string.as_str());
    info!("Report failure for external task at {}", endpoint);

    let client = reqwest::Client::new();
    let request = build_authenticated_post(
        &client,
        endpoint.clone(),
        config.username(),
        config.password(),
    )
    .json(&FailureRequest {
        worker_id: config.id(),
        error_message,
        error_details,
        retries,
        retry_timeout: retry_timeout_ms,
    });

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
        error!("Failure report failed: status={} body={} ", status, body);
        return Err(format!("Failure report failed with status {status}").into());
    }

    trace!("Task '{}' failure reported", external_task_id);
    Ok(())
}

pub async fn report_bpmn_error(
    config: &ConfigParams,
    external_task_id: &str,
    error_code: &str,
    error_message: Option<&str>,
    variables: Option<OutputVariables>,
) -> Result<(), Box<dyn Error>> {
    let mut endpoint = config.url().clone();
    let path_string = format!(
        "engine-rest/external-task/{}/bpmnError",
        external_task_id
    );
    endpoint.set_path(path_string.as_str());
    info!("Report BPMN error for external task at {}", endpoint);

    let client = reqwest::Client::new();
    let request = build_authenticated_post(
        &client,
        endpoint.clone(),
        config.username(),
        config.password(),
    )
    .json(&BpmnErrorRequest {
        worker_id: config.id(),
        error_code,
        error_message,
        variables,
    });

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
        error!("BPMN error report failed: status={} body={} ", status, body);
        return Err(format!("BPMN error report failed with status {status}").into());
    }

    trace!("Task '{}' BPMN error reported", external_task_id);
    Ok(())
}
