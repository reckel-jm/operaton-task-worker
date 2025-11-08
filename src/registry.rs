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

#[cfg(test)]
mod tests {
    use super::*;

    // Define a dummy handler via the attribute macro and assert it is discoverable
    #[operaton_task_worker_macros::task_handler(name = "__test_handler__example__")]
    fn test_handler(_input: &crate::types::InputVariables) -> Result<crate::types::OutputVariables, Box<dyn std::error::Error>> {
        Ok(std::collections::HashMap::new())
    }

    #[test]
    fn registry_finds_macro_registered_handler() {
        // Ensure that the handler registered by the macro is present
        let names = all_names();
        assert!(names.contains(&"__test_handler__example__"));
        let f = find("__test_handler__example__").expect("handler should be registered");
        // Call it with empty input to ensure function pointer is valid
        let input: crate::types::InputVariables = std::collections::HashMap::new();
        let out = f(&input).expect("handler should return Ok");
        assert!(out.is_empty());
    }
}
