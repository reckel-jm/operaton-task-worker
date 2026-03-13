//! The macro tests are located in this file.

use operaton_task_worker_macros::task_handler;
use operaton_task_worker::types::{InputVariables, OutputVariables, out_string, out_json};
use operaton_task_worker::registry;


// Define a dummy handler via the attribute macro and assert it is discoverable
#[task_handler(name = "__test_handler__example__")]
fn test_handler(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    Ok(std::collections::HashMap::new())
}


#[task_handler(name = "example_echo")]
pub fn example_echo(input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("workerResponse".to_string(), out_string("ok"));

    // Return a summary JSON of the input variable names
    let keys: Vec<&String> = input.keys().collect();
    let summary = serde_json::json!({ "keys": keys });
    out.insert("summary".to_string(), out_json(&summary));

    Ok(out)
}

#[task_handler(name = "ServiceTask_GetScannedFiles")]
pub fn get_scanned_files(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("FILENAMES".to_string(), out_string("TEST"));
    Ok(out)
}

// Handler registered only by topic
#[task_handler(topic = "__test_topic_only__")]
pub fn topic_only_handler(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("source".to_string(), out_string("topic"));
    Ok(out)
}

// Handler registered by both name and topic
#[task_handler(name = "__test_name_and_topic_name__", topic = "__test_name_and_topic_topic__")]
pub fn name_and_topic_handler(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("source".to_string(), out_string("name_and_topic"));
    Ok(out)
}

#[test]
fn test_find_handler_by_name() {
    assert!(registry::find("__test_handler__example__").is_some());
    assert!(registry::find("nonexistent_handler").is_none());
}

#[test]
fn test_find_handler_by_topic() {
    assert!(registry::find_by_topic("__test_topic_only__").is_some());
    assert!(registry::find_by_topic("nonexistent_topic").is_none());
}

#[test]
fn test_find_handler_by_name_and_topic() {
    // A handler with both name and topic can be found by either
    assert!(registry::find("__test_name_and_topic_name__").is_some());
    assert!(registry::find_by_topic("__test_name_and_topic_topic__").is_some());
}

#[test]
fn test_topic_only_handler_not_found_by_name() {
    // A topic-only handler should not appear in name-based lookup
    assert!(registry::find("__test_topic_only__").is_none());
}

#[test]
fn test_name_only_handler_not_found_by_topic() {
    // A name-only handler should not appear in topic-based lookup
    assert!(registry::find_by_topic("__test_handler__example__").is_none());
}
