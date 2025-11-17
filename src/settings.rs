use config::Config;

/// Loads the configuration into a [ConfigParams] struct. The function may panic, but it should not
/// happen because [ConfigParams] provides default values for all configured entries.
pub fn load_config_from_env(env_prefix: &str) -> ConfigParams {
    let settings = Config::builder()
        .add_source(config::Environment::with_prefix(env_prefix))
        .build()
        .unwrap();

    settings.try_deserialize().unwrap()
}

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

    /// The lock duration in milliseconds for external task locking
    #[serde(default = "default_lock_duration")]
    lock_duration: u64,
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

    pub fn lock_duration(&self) -> u64 { self.lock_duration }

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

    pub fn with_lock_duration(self, lock_duration: u64) -> Self {
        let mut cloned_self = self.clone();
        cloned_self.lock_duration = lock_duration;
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
            lock_duration: default_lock_duration(),
        }
    }
}

fn default_url() -> Url {
    Url::parse("http://localhost:8080").unwrap()
}

/// The default poll interval in milliseconds
fn default_poll_interval() -> usize { 500 }

fn default_task_worker_id() -> String { "operaton_task_worker".to_string() }

fn default_lock_duration() -> u64 { 60_000 }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_params_builder_pattern() {
        let config = ConfigParams::default()
            .with_url(Url::parse("http://localhost:8080").unwrap())
            .with_auth("user".to_string(), "pass".to_string())
            .with_poll_interval(1000)
            .with_worker_id("operaton_task_worker".to_string())
            .with_lock_duration(12_345);

        assert_eq!(config.url(), &Url::parse("http://localhost:8080").unwrap());
        assert_eq!(config.username(), "user");
        assert_eq!(config.password(), "pass");
        assert_eq!(config.poll_interval(), 1000);
        assert_eq!(config.id(), "operaton_task_worker");
        assert_eq!(config.lock_duration(), 12_345);
    }

    #[test]
    fn test_default_lock_duration() {
        let cfg = ConfigParams::default();
        assert_eq!(cfg.lock_duration(), default_lock_duration());
    }
}