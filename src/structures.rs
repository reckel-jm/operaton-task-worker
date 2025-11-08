use serde::{Deserialize, Serialize};

use url::Url;

/// The struct contains all config params for running the task worker
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigParams {
    /// The URL where operaton can be found
    #[serde(default = "default_url")]
    url: Url,

    /// The username for authenticating with the REST API
    /// - If empty, no authentication will be used (default).
    #[serde(default = "String::new")]
    username: String,

    /// The password for authenticating with the REST API
    /// - If empty, no authentication will be used (default).
    #[serde(default = "String::new")]
    password: String,

    /// The interval in milliseconds for polling the Operaton Task Worker for new tasks
    #[serde(default = "default_poll_interval")]
    poll_interval: usize,

    #[serde(default = "default_task_worker_id")]
    /// The task worker id which will be registered with Operaton
    id: String,
}

impl ConfigParams {
    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn poll_interval(&self) -> usize {
        self.poll_interval
    }
}

fn default_url() -> Url {
    Url::parse("http://localhost:8080").unwrap()
}

/// The default poll interval in milliseconds
fn default_poll_interval() -> usize { 500 }

fn default_task_worker_id() -> String { "operaton_task_worker".to_string() }

/// An Operaton Service Task with its description elements
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServiceTask {
    /// The id of the Service Task (called `activityId` in Operaton)
    activity_id: String,
    process_instance_id: String,
    suspended: bool,
    topic_name: String,
    priority: usize,
    business_key: String,
    worker_id: Option<String>,
}

impl ServiceTask {
    pub fn activity_id(&self) -> &str {
        &self.activity_id
    }

    pub fn process_instance_id(&self) -> &str {
        &self.process_instance_id
    }

    pub fn suspended(&self) -> bool {
        self.suspended
    }

    pub fn topic_name(&self) -> &str {
        &self.topic_name
    }

    pub fn priority(&self) -> usize {
        self.priority
    }

    pub fn business_key(&self) -> &str {
        &self.business_key
    }
}