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

    pub fn id(&self) -> &str { &self.id }

    pub fn with_url(self, url: Url) -> Self {
        let mut cloned_self = self.clone();
        cloned_self.url = url;
        cloned_self
    }

    pub fn with_auth(self, username: String, password: String) -> Self {
        let mut cloned_self = self.clone();
        cloned_self.username = username;
        cloned_self.password = password;
        cloned_self
    }

    pub fn with_poll_interval(self, poll_interval: usize) -> Self {
        let mut cloned_self = self.clone();
        cloned_self.poll_interval = poll_interval;
        cloned_self
    }

    pub fn with_worker_id(self, id: String) -> Self {
        let mut cloned_self = self.clone();
        cloned_self.id = id;
        cloned_self
    }
}

impl Default for ConfigParams {
    fn default() -> Self {
        Self {
            url: default_url(),
            username: String::new(),
            password: String::new(),
            poll_interval: default_poll_interval(),
            id: default_task_worker_id(),
        }
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
    /// The external task id (Camunda/Operaton external task id)
    id: String,
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
    pub fn id(&self) -> &str { &self.id }

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_params_builder_pattern() {
        let config = ConfigParams::default()
            .with_url(Url::parse("http://localhost:8080").unwrap())
            .with_auth("user".to_string(), "pass".to_string())
            .with_poll_interval(1000)
            .with_worker_id("operaton_task_worker".to_string());

        assert_eq!(config.url(), &Url::parse("http://localhost:8080").unwrap());
        assert_eq!(config.username(), "user");
        assert_eq!(config.password(), "pass");
        assert_eq!(config.poll_interval(), 1000);
        assert_eq!(config.id(), "operaton_task_worker");
    }
}