use crate::types::ExternalTaskFn;

pub struct Handler {
    pub name: Option<&'static str>,
    pub topic: Option<&'static str>,
    pub func: ExternalTaskFn,
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

