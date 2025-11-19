use crate::types::ExternalTaskFn;

pub struct Handler {
    pub name: &'static str,
    pub topic: Option<&'static str>,
    pub func: ExternalTaskFn,
}

inventory::collect!(Handler);

pub fn find(name: &str) -> Option<ExternalTaskFn> {
    for h in inventory::iter::<Handler> {
        if h.name == name {
            return Some(h.func);
        }
    }
    None
}

/// Find a handler by either matching topic (if any handler declares one) or by activity id (name)
pub fn find_for_task(activity_id: &str, topic_name: &str) -> Option<ExternalTaskFn> {
    // Prefer topic match when declared
    for h in inventory::iter::<Handler> {
        if let Some(t) = h.topic {
            if t == topic_name {
                return Some(h.func);
            }
        }
    }
    // Fallback to activity id/name
    find(activity_id)
}

pub fn all_names() -> Vec<&'static str> {
    inventory::iter::<Handler>.into_iter().map(|h| h.name).collect()
}

pub fn topics() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = inventory::iter::<Handler>
        .into_iter()
        .filter_map(|h| h.topic)
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

pub fn has_nontopic_handlers() -> bool {
    inventory::iter::<Handler>.into_iter().any(|h| h.topic.is_none())
}

