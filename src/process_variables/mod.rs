use std::collections::HashMap;
/// This module handles Process Variables and their different kinds

use serde::*;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonValue {
    data_format_name: String,

    value: serde_json::Value,

    string: bool,

    object: bool,

    boolean: bool,

    number: bool,

    array: bool,

    #[serde(rename = "null")]
    null_val: bool,

    node_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonVar {
    #[serde(rename = "value")]
    pub json_value: JsonValue,

    #[serde(rename = "valueInfo")]
    pub value_info: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoolVar {
    pub value: bool,
    
    #[serde(rename = "valueInfo")]
    pub value_info: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StringVar {
    pub value: String,

    #[serde(rename = "valueInfo")]
    pub value_info: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub enum ProcessInstanceVariable {
    Json(JsonVar),
    Boolean(BoolVar),
    String(StringVar),
}

/// This represents an entry of the original JSON
#[derive(Deserialize)]
pub struct Entry {
    #[serde(rename = "type")]
    typ: String,

    value: serde_json::Value,

    #[serde(rename = "valueInfo")]
    value_info: HashMap<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for ProcessInstanceVariable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = HashMap::<String, Entry>::deserialize(deserializer)?;

        // We expect only one entry in practice, but we'll take the first valid one
        // Or collect all into Vec<Var> if you want multiple
        for (_, entry) in map {
            return match entry.typ.as_str() {
                "Json" => {
                    let json_var = JsonVar {
                        json_value: serde_json::from_value(entry.value).map_err(serde::de::Error::custom)?,
                        value_info: entry.value_info,
                    };
                    Ok(ProcessInstanceVariable::Json(json_var))
                }
                "Boolean" => {
                    let bool_var = BoolVar {
                        value: serde_json::from_value(entry.value).map_err(serde::de::Error::custom)?,
                        value_info: entry.value_info,
                    };
                    Ok(ProcessInstanceVariable::Boolean(bool_var))
                },
                "String" => {
                    let string_var = StringVar {
                        value: serde_json::from_value(entry.value).map_err(serde::de::Error::custom)?,
                        value_info: entry.value_info,
                    };
                    Ok(ProcessInstanceVariable::String(string_var))
                },
                _ => Err(serde::de::Error::custom(format!("unknown type: {}", entry.typ))),
            };
        }

        Err(serde::de::Error::custom("no valid entries found"))
    }
}

pub fn parse_process_instance_variables(json_str: &str) -> HashMap<String, ProcessInstanceVariable> {
    let map: HashMap<String, Entry> = serde_json::from_str(json_str).unwrap();
    let mut result = HashMap::new();
    for (key, entry) in map {
        let var = match entry.typ.as_str() {
            "Json" => (key, ProcessInstanceVariable::Json(JsonVar {
                json_value: serde_json::from_value(entry.value).unwrap(),
                value_info: entry.value_info,
            })),
            "Boolean" => (key, ProcessInstanceVariable::Boolean(BoolVar {
                value: serde_json::from_value(entry.value).unwrap(),
                value_info: entry.value_info,
            })),
            "String" => (key, ProcessInstanceVariable::String(StringVar {
                value: serde_json::from_value(entry.value).unwrap(),
                value_info: entry.value_info,
            })),
            _ => continue,
        };
        result.insert(var.0, var.1);
    }
    result
}

#[cfg(test)]
mod test {
    use crate::process_variables::parse_process_instance_variables;

    #[test]
    fn test_module_parsing() {
        let response_string: &str = "{\"checklist_vj3ler\":{\"type\":\"Json\",\"value\":{\"dataFormatName\":\"application/json\",\"value\":false,\"string\":false,\"object\":false,\"boolean\":false,\"number\":false,\"array\":true,\"null\":false,\"nodeType\":\"ARRAY\"},\"valueInfo\":{}},\"checkbox_6ow5yg\":{\"type\":\"Boolean\",\"value\":true,\"valueInfo\":{}}}";
        let variables = parse_process_instance_variables(response_string);
        dbg!(&variables);
        assert!(variables.len() > 0)
    }
}