use crate::types::ExternalTaskFn;

pub struct Handler {
    name: Option<&'static str>,
    topic: Option<&'static str>,
    func: ExternalTaskFn,
}

impl Handler {
    /// Construct a new handler. At least one of `name` or `topic` must be `Some`.
    pub const fn new(
        name: Option<&'static str>,
        topic: Option<&'static str>,
        func: ExternalTaskFn,
    ) -> Handler {
        debug_assert!(
            name.is_some() || topic.is_some(),
            "Handler must have at least a name or a topic"
        );
        Handler { name, topic, func }
    }
}

inventory::collect!(Handler);

/// Find a handler by task name (activity ID).
pub fn find(name: &str) -> Option<ExternalTaskFn> {
    for h in inventory::iter::<Handler> {
        if h.name == Some(name) {
            return Some(h.func);
        }
    }
    None
}

/// Find a handler by topic name.
pub fn find_by_topic(topic: &str) -> Option<ExternalTaskFn> {
    for h in inventory::iter::<Handler> {
        if h.topic == Some(topic) {
            return Some(h.func);
        }
    }
    None
}

/// Returns the names of all registered handlers that have a name set.
pub fn all_names() -> Vec<&'static str> {
    inventory::iter::<Handler>.into_iter().filter_map(|h| h.name).collect()
}

/// Returns the topics of all registered handlers that have a topic set.
pub fn all_topics() -> Vec<&'static str> {
    inventory::iter::<Handler>.into_iter().filter_map(|h| h.topic).collect()
}

