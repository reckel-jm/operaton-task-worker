use std::collections::HashMap;

use crate::process_variables::ProcessInstanceVariable;

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

pub type InputVariables = HashMap<String, ProcessInstanceVariable>;
pub type OutputVariables = HashMap<String, OutVariable>;
pub type ExternalTaskFn = fn(&InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>>;

pub fn out_string(value: impl Into<String>) -> OutVariable {
    OutVariable {
        value: serde_json::Value::String(value.into()),
        typ: "String".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

#[allow(dead_code)]
pub fn out_bool(value: bool) -> OutVariable {
    OutVariable {
        value: serde_json::Value::Bool(value),
        typ: "Boolean".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

#[allow(dead_code)]
pub fn out_integer(value: i32) -> OutVariable {
    OutVariable {
        value: serde_json::Value::Number(serde_json::Number::from(value)),
        typ: "Integer".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

#[allow(dead_code)]
pub fn out_long(value: i64) -> OutVariable {
    OutVariable {
        value: serde_json::Value::Number(serde_json::Number::from(value)),
        typ: "Long".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

#[allow(dead_code)]
pub fn out_double(value: f64) -> OutVariable {
    OutVariable {
        value: serde_json::json!(value),
        typ: "Double".to_string(),
        value_info: std::collections::HashMap::new(),
    }
}

pub fn out_json(value: &serde_json::Value) -> OutVariable {
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

// A typed error that signals a BPMN error should be raised instead of a technical failure.
#[derive(Debug, Clone)]
pub struct BpmnError {
    pub code: String,
    pub message: Option<String>,
}

impl BpmnError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: Some(message.into()) }
    }
    pub fn with_code_only(code: impl Into<String>) -> Self {
        Self { code: code.into(), message: None }
    }
}

impl std::fmt::Display for BpmnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            Some(m) => write!(f, "BPMN Error {}: {}", self.code, m),
            None => write!(f, "BPMN Error {}", self.code),
        }
    }
}

impl std::error::Error for BpmnError {}
