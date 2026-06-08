//! The macro tests are located in this file.

use operaton_task_worker_macros::task_handler;
use operaton_task_worker::types::{InputVariables, OutputVariables, out_string, out_json};


// Define a dummy handler via the attribute macro and assert it is discoverable
#[task_handler(name = "__test_handler__example__")]
fn test_handler(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    Ok(std::collections::HashMap::new())
}


#[task_handler(topic = "example_echo")]
pub fn example_echo(input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("workerResponse".to_string(), out_string("ok"));

    // Return a summary JSON of the input variable names
    let keys: Vec<&String> = input.keys().collect();
    let summary = serde_json::json!({ "keys": keys });
    out.insert("summary".to_string(), out_json(&summary));

    Ok(out)
}

#[task_handler(topic = "ServiceTask_GetScannedFiles")]
pub fn get_scanned_files(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
    let mut out: OutputVariables = std::collections::HashMap::new();
    out.insert("FILENAMES".to_string(), out_string("TEST"));
    Ok(out)
}