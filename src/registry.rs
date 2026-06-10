use crate::types::ExternalTaskFn;

pub struct Handler {
    pub name: Option<&'static str>,
    pub topic: Option<&'static str>,
    pub func: ExternalTaskFn,
}

inventory::collect!(Handler);

/// Find a handler strictly by name (activity id)
pub fn find(name: &str) -> Option<ExternalTaskFn> {
    for h in inventory::iter::<Handler> {
        // Safely compare Option<&str> with Some(&str)
        if h.name == Some(name) {
            return Some(h.func);
        }
    }
    None
}

/// Find a handler by either matching topic (if declared) or by activity id (name)
pub fn find_for_task(activity_id: &str, topic_name: &str) -> Option<ExternalTaskFn> {
    // Prefer topic match when declared
    for h in inventory::iter::<Handler> {
        if h.topic == Some(topic_name) {
            return Some(h.func);
        }
    }

    // Fallback to activity id/name
    find(activity_id)
}

/// Retrieve all registered names, ignoring handlers that only have a topic
pub fn all_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = inventory::iter::<Handler>
        .into_iter()
        // Filter out None values, keeping only valid names
        .filter_map(|h| h.name)
        .collect();

    // Sort and deduplicate for a clean, predictable output
    v.sort_unstable();
    v.dedup();
    v
}

/// Retrieve all registered topics, ignoring handlers that only have a name
pub fn topics() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = inventory::iter::<Handler>
        .into_iter()
        .filter_map(|h| h.topic)
        .collect();

    v.sort_unstable();
    v.dedup();
    v
}

/// Check if there are handlers registered without a topic
pub fn has_nontopic_handlers() -> bool {
    inventory::iter::<Handler>.into_iter().any(|h| h.topic.is_none())
}