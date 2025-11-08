use crate::types::ExternalTaskFn;

pub struct Handler {
    pub name: &'static str,
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

pub fn all_names() -> Vec<&'static str> {
    inventory::iter::<Handler>.into_iter().map(|h| h.name).collect()
}
