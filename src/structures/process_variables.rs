use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectVar {
    pub value: serde_json::Value,

    #[serde(rename = "valueInfo")]
    pub value_info: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub enum ProcessInstanceVariable {
    Json(JsonVar),
    Object(ObjectVar),
    Boolean(BoolVar),
    String(StringVar),
}

impl ProcessInstanceVariable {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ProcessInstanceVariable::Boolean(b) => Some(b.value),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ProcessInstanceVariable::String(s) => Some(&s.value),
            _ => None,
        }
    }
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            ProcessInstanceVariable::Json(j) => Some(&j.json_value.value),
            ProcessInstanceVariable::Object(o) => Some(&o.value),
            _ => None,
        }
    }

    pub fn as_typed<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match self {
            ProcessInstanceVariable::Json(j) => serde_json::from_value(j.json_value.value.clone()),
            ProcessInstanceVariable::Object(o) => serde_json::from_value(o.value.clone()),
            ProcessInstanceVariable::Boolean(b) => serde_json::from_value(serde_json::Value::Bool(b.value)),
            ProcessInstanceVariable::String(s) => {
                serde_json::from_value(serde_json::Value::String(s.value.clone()))
            }
        }
    }

    pub fn as_object_string(&self) -> Option<String> {
        match self {
            ProcessInstanceVariable::Object(o) => match &o.value {
                serde_json::Value::String(raw) => Some(raw.clone()),
                other => Some(other.to_string()),
            },
            _ => None,
        }
    }
}

fn is_json_serialized_object(value_info: &HashMap<String, serde_json::Value>) -> bool {
    let data_format_is_json = value_info
        .get("serializationDataFormat")
        .and_then(serde_json::Value::as_str)
        .map(|fmt| fmt.eq_ignore_ascii_case("application/json"))
        .unwrap_or(false);

    let is_java_array_list = value_info
        .get("objectTypeName")
        .and_then(serde_json::Value::as_str)
        .map(|type_name| type_name.starts_with("java.util.ArrayList"))
        .unwrap_or(false);

    data_format_is_json || is_java_array_list
}

fn parse_object_json_value(
    value: serde_json::Value,
    value_info: &HashMap<String, serde_json::Value>,
) -> serde_json::Value {
    if !is_json_serialized_object(value_info) {
        return value;
    }

    match value {
        serde_json::Value::String(raw) => serde_json::from_str(&raw)
            .unwrap_or_else(|_| serde_json::Value::String(raw)),
        other => other,
    }
}

/// This represents an entry of the original JSON
#[derive(Deserialize)]
pub struct Entry {
    #[serde(rename = "type")]
    typ: String,

    #[serde(default)]
    #[allow(dead_code)]
    name: String,

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
                "Object" => {
                    let object_var = ObjectVar {
                        value: parse_object_json_value(entry.value, &entry.value_info),
                        value_info: entry.value_info,
                    };
                    Ok(ProcessInstanceVariable::Object(object_var))
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
    // According to Camunda 7/Operaton, the variable endpoint usually returns an object map of
    // name -> { type, value, valueInfo }. However, sometimes multiple JSON values can be returned
    // as a JSON sequence (concatenated JSON values or an array). This function now handles:
    // 1) A single JSON object map
    // 2) A JSON array of such maps
    // 3) A JSON array of entries (flat list with `name` inside)
    // 4) A concatenated JSON sequence of such maps or entries

    // Helper to convert an Entry into our enum and insert into result map
    fn insert_entry(result: &mut HashMap<String, ProcessInstanceVariable>, name: String, entry: Entry) {
        let parsed_var = match entry.typ.as_str() {
            "Json" => ProcessInstanceVariable::Json(JsonVar {
                json_value: serde_json::from_value(entry.value).unwrap_or_else(|e| {
                    println!("Failed to parse JsonVar value for {}: {:#?}", name, e);
                    // Fallback empty JSON details
                    JsonValue {
                        data_format_name: String::new(),
                        value: serde_json::Value::Null,
                        string: false,
                        object: false,
                        boolean: false,
                        number: false,
                        array: false,
                        null_val: true,
                        node_type: String::new(),
                    }
                }),
                value_info: entry.value_info,
            }),
            "Object" => ProcessInstanceVariable::Object(ObjectVar {
                value: parse_object_json_value(entry.value, &entry.value_info),
                value_info: entry.value_info,
            }),
            "Boolean" => ProcessInstanceVariable::Boolean(BoolVar {
                value: serde_json::from_value(entry.value).unwrap_or(false),
                value_info: entry.value_info,
            }),
            "String" => ProcessInstanceVariable::String(StringVar {
                value: serde_json::from_value(entry.value).unwrap_or_default(),
                value_info: entry.value_info,
            }),
            _ => return,
        };
        result.insert(name, parsed_var);
    }

    let mut result: HashMap<String, ProcessInstanceVariable> = HashMap::new();

    // Strategy 1: Try a single object map
    if let Ok(parsed_map) = serde_json::from_str::<HashMap<String, Entry>>(json_str) {
        for (name, entry) in parsed_map {
            insert_entry(&mut result, name, entry);
        }
        return result;
    }

    // Strategy 2: Try an array of entries (flat list with `name` field)
    if let Ok(entries) = serde_json::from_str::<Vec<Entry>>(json_str) {
        for entry in entries.into_iter() {
            let name = if !entry.name.is_empty() { entry.name.clone() } else { continue };
            insert_entry(&mut result, name, entry);
        }
        return result;
    }

    // Strategy 3: Try an array of object maps
    if let Ok(parsed_vec) = serde_json::from_str::<Vec<HashMap<String, Entry>>>(json_str) {
        for map in parsed_vec.into_iter() {
            for (name, entry) in map {
                insert_entry(&mut result, name, entry);
            }
        }
        return result;
    }

    // Strategy 4a: Stream/sequence of concatenated Entry values
    let deser_entries = serde_json::Deserializer::from_str(json_str);
    let mut stream_entries = deser_entries.into_iter::<Entry>();
    let mut any_parsed = false;
    while let Some(next) = stream_entries.next() {
        match next {
            Ok(entry) => {
                if !entry.name.is_empty() {
                    let name = entry.name.clone();
                    insert_entry(&mut result, name, entry);
                    any_parsed = true;
                }
            }
            Err(e) => {
                println!("Error while parsing JSON Entry sequence chunk: {:#?}", e);
                break;
            }
        }
    }
    if any_parsed {
        return result;
    }

    // Strategy 4b: Stream/sequence of concatenated map values
    let deser_maps = serde_json::Deserializer::from_str(json_str);
    let mut stream_maps = deser_maps.into_iter::<HashMap<String, Entry>>();
    while let Some(next) = stream_maps.next() {
        match next {
            Ok(map) => {
                any_parsed = true;
                for (name, entry) in map {
                    insert_entry(&mut result, name, entry);
                }
            }
            Err(e) => {
                // Stop streaming on error; we will report below if nothing parsed
                println!("Error while parsing JSON map sequence chunk: {:#?}", e);
                break;
            }
        }
    }

    if !any_parsed {
        println!("Error while parsing \"{}\", ignoring it for now.", json_str);
    }

    result
}

#[cfg(test)]
mod test {
    use crate::structures::process_variables::{parse_process_instance_variables, ProcessInstanceVariable};
    use serde::Deserialize;

    #[test]
    fn test_module_parsing() {
        let response_string: &str = "{\"checklist_vj3ler\":{\"type\":\"Json\",\"value\":{\"dataFormatName\":\"application/json\",\"value\":false,\"string\":false,\"object\":false,\"boolean\":false,\"number\":false,\"array\":true,\"null\":false,\"nodeType\":\"ARRAY\"},\"valueInfo\":{}},\"checkbox_6ow5yg\":{\"type\":\"Boolean\",\"value\":true,\"valueInfo\":{}}}";
        let variables = parse_process_instance_variables(response_string);
        dbg!(&variables);
        assert!(!variables.is_empty())
    }

    #[test]
    fn test_module_parsing_complex_json_sequence() {
        let response_string: &str = "[{\"type\":\"String\",\"value\":\"5x Vier Jahreszeiten\",\"valueInfo\":{},\"id\":\"f9bac09f-c5df-11f0-94e9-0242c0a80103\",\"name\":\"pizza_wishlist\",\"processDefinitionId\":\"OrderPizza:3:f2d157ce-c5df-11f0-94e9-0242c0a80103\",\"processInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"executionId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"caseInstanceId\":null,\"caseExecutionId\":null,\"taskId\":null,\"batchId\":null,\"activityInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"errorMessage\":null,\"tenantId\":null},{\"type\":\"String\",\"value\":\"JA\",\"valueInfo\":{},\"id\":\"f9bac0a2-c5df-11f0-94e9-0242c0a80103\",\"name\":\"mehrheit_will_pizza\",\"processDefinitionId\":\"OrderPizza:3:f2d157ce-c5df-11f0-94e9-0242c0a80103\",\"processInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"executionId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"caseInstanceId\":null,\"caseExecutionId\":null,\"taskId\":null,\"batchId\":null,\"activityInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"errorMessage\":null,\"tenantId\":null},{\"type\":\"String\",\"value\":\"JA\",\"valueInfo\":{},\"id\":\"f9bae7b5-c5df-11f0-94e9-0242c0a80103\",\"name\":\"MehrheitWillPizza\",\"processDefinitionId\":\"OrderPizza:3:f2d157ce-c5df-11f0-94e9-0242c0a80103\",\"processInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"executionId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"caseInstanceId\":null,\"caseExecutionId\":null,\"taskId\":null,\"batchId\":null,\"activityInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"errorMessage\":null,\"tenantId\":null},{\"type\":\"String\",\"value\":\"5x Vier Jahreszeiten\",\"valueInfo\":{},\"id\":\"f9bae7b7-c5df-11f0-94e9-0242c0a80103\",\"name\":\"Pizzawuensche\",\"processDefinitionId\":\"OrderPizza:3:f2d157ce-c5df-11f0-94e9-0242c0a80103\",\"processInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"executionId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"caseInstanceId\":null,\"caseExecutionId\":null,\"taskId\":null,\"batchId\":null,\"activityInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"errorMessage\":null,\"tenantId\":null},{\"type\":\"String\",\"value\":\"5x Vier Jahreszeiten\",\"valueInfo\":{},\"id\":\"f9bb0ecc-c5df-11f0-94e9-0242c0a80103\",\"name\":\"Bestellungen\",\"processDefinitionId\":\"OrderPizza:3:f2d157ce-c5df-11f0-94e9-0242c0a80103\",\"processInstanceId\":\"f2d4da42-c5df-11f0-94e9-0242c0a80103\",\"executionId\":\"f9bb0eca-c5df-11f0-94e9-0242c0a80103\",\"caseInstanceId\":null,\"caseExecutionId\":null,\"taskId\":null,\"batchId\":null,\"activityInstanceId\":\"ServiceTask_OrderPizza:f9bb0ecb-c5df-11f0-94e9-0242c0a80103\",\"errorMessage\":null,\"tenantId\":null}]";
        let variables = parse_process_instance_variables(response_string);
        dbg!(&variables);
        assert!(!(variables.is_empty()))
    }

    #[test]
    fn test_module_parsing_invalid() {
        let response_string: &str = "{\"invalid\":}";

        let variables = parse_process_instance_variables(response_string);
        dbg!(&variables);
        assert!(variables.is_empty())
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct WishlistItem {
        name: String,
        amount: u32,
    }

    #[test]
    fn test_module_parsing_object_arraylist_json() {
        let response_string = r#"{
            "wishlist": {
                "type": "Object",
                "value": "[{\"name\":\"Vier Jahreszeiten\",\"amount\":5}]",
                "valueInfo": {
                    "serializationDataFormat": "application/json",
                    "objectTypeName": "java.util.ArrayList"
                }
            }
        }"#;

        let variables = parse_process_instance_variables(response_string);
        let parsed = variables
            .get("wishlist")
            .expect("wishlist variable missing")
            .as_typed::<Vec<WishlistItem>>()
            .expect("wishlist should deserialize into Vec<WishlistItem>");

        assert_eq!(
            parsed,
            vec![WishlistItem {
                name: "Vier Jahreszeiten".to_string(),
                amount: 5,
            }]
        );
    }

    #[test]
    fn test_object_with_non_json_data_format_stays_string() {
        let response_string = r#"{
            "opaque": {
                "type": "Object",
                "value": "not-json",
                "valueInfo": {
                    "serializationDataFormat": "application/xml",
                    "objectTypeName": "java.util.ArrayList"
                }
            }
        }"#;

        let variables = parse_process_instance_variables(response_string);
        let opaque = variables.get("opaque").expect("opaque variable missing");
        match opaque {
            ProcessInstanceVariable::Object(obj) => {
                assert_eq!(obj.value, serde_json::Value::String("not-json".to_string()));
            }
            _ => panic!("expected Object variant"),
        }
    }

    #[test]
    fn test_object_as_string_from_arraylist_json() {
        let response_string = r#"{
            "wishlist": {
                "type": "Object",
                "value": "[{\"name\":\"Margherita\",\"amount\":2}]",
                "valueInfo": {
                    "serializationDataFormat": "application/json",
                    "objectTypeName": "java.util.ArrayList"
                }
            }
        }"#;

        let variables = parse_process_instance_variables(response_string);
        let raw = variables
            .get("wishlist")
            .expect("wishlist variable missing")
            .as_object_string()
            .expect("wishlist object should be convertible to string");

        let parsed_raw: serde_json::Value =
            serde_json::from_str(&raw).expect("object string should be valid JSON");
        let expected: serde_json::Value = serde_json::json!([
            {
                "name": "Margherita",
                "amount": 2
            }
        ]);

        assert_eq!(parsed_raw, expected);
    }
}