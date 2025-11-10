//! This module includes the functions for the main polling loop

use std::collections::HashMap;
use log::{debug, error, info, trace, warn};
use crate::{api, registry};
use crate::process_variables::ProcessInstanceVariable;
use crate::structures::ConfigParams;
use crate::types::BpmnError;

pub async fn start_polling_loop(config: ConfigParams) {

    env_logger::init();

    info!("Load Operaton Task Worker with configuration: {:#?}", config);

    if config.username().is_empty() || config.password().is_empty() {
        warn!("No authentication set up. Operaton should be protected by authentication in productive use.");
    }

    trace!("Enter the main loop");

    loop {
        match api::get_open_service_tasks(&config).await {
            Ok(service_tasks) => {
                info!(
                    "We received {} open external Service Tasks from Operaton.",
                    service_tasks.len()
                );

                for service_task in service_tasks {
                    // Try to lock the specific external task and read its input variables
                    if let Err(err) = api::lock_external_task(&config, service_task.id(), 60_000).await {
                        warn!("Could not lock task {}: {:#?}", service_task.id(), err);
                        continue;
                    }

                    let input_vars: HashMap<String, ProcessInstanceVariable> = api::get_process_instance_variables(&config, service_task.process_instance_id()).await.unwrap_or_else(|err| {
                        error!("Error while fetching external task variables: {:#?}", err);
                        HashMap::new()
                    });
                    trace!("External task variables for {} => {:#?}", service_task.id(), input_vars);

                    if let Some(function) = registry::find(service_task.activity_id()) {
                        debug!("Executing function for Service Task: {:#?}", service_task);
                        match function(&input_vars) {
                            Ok(output_vars) => {
                                if let Err(err) = api::complete_external_task(&config, service_task.id(), output_vars).await {
                                    error!("Could not complete external task {}: {:#?}", service_task.id(), err);
                                } else {
                                    info!("Completed external task {}", service_task.id());
                                }
                            }
                            Err(err) => {
                                error!("Execution of function for Service Task {} failed: {:#?}", service_task.id(), err);
                                // Distinguish BPMN business errors from technical failures
                                if let Some(bpmn) = err.downcast_ref::<BpmnError>() {
                                    if let Err(e) = api::report_bpmn_error(
                                        &config,
                                        service_task.id(),
                                        &bpmn.code,
                                        bpmn.message.as_deref(),
                                        None,
                                    ).await {
                                        error!("Could not report BPMN error for task {}: {:#?}", service_task.id(), e);
                                    }
                                } else {
                                    if let Err(e) = api::report_external_task_failure(
                                        &config,
                                        service_task.id(),
                                        &err.to_string(),
                                        None,
                                        0,
                                        0,
                                    ).await {
                                        error!("Could not report failure for task {}: {:#?}", service_task.id(), e);
                                    }
                                }
                            }
                        }
                    } else {
                        warn!("No function found for Service Task: {:#?}. SKIP.", service_task.activity_id());
                    }
                };
            },
            Err(error) => error!("We were unable to receive and parse any Service Tasks. Error: {:#}", error)
        }

        // Wait for the in `config.poll_interval` milliseconds
        tokio::time::sleep(tokio::time::Duration::from_millis(config.poll_interval() as u64)).await;
    }
}