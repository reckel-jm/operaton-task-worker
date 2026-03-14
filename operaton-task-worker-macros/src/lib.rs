/*
The crate [operaton_task_worker_macros] provides a proc-macro attribute macro to register an external task handler function with a name (activityId/topic).
*/

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, Token, LitStr};
use syn::parse::{Parse, ParseStream};
use proc_macro_crate::{crate_name, FoundCrate};

/// Arguments parsed from the `#[task_handler(...)]` attribute.
/// At least one of `name` or `topic` must be provided.
struct TaskHandlerArgs {
    name: Option<String>,
    topic: Option<String>,
}

impl Parse for TaskHandlerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut topic = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let _: Token![=] = input.parse()?;
            let value: LitStr = input.parse()?;

            match key.to_string().as_str() {
                "name" => {
                    if name.is_some() {
                        return Err(syn::Error::new(
                            key.span(),
                            "Duplicate 'name' argument in #[task_handler] attribute",
                        ));
                    }
                    name = Some(value.value());
                }
                "topic" => {
                    if topic.is_some() {
                        return Err(syn::Error::new(
                            key.span(),
                            "Duplicate 'topic' argument in #[task_handler] attribute",
                        ));
                    }
                    topic = Some(value.value());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "Unknown attribute key '{}'. Expected 'name' or 'topic'.",
                            other
                        ),
                    ))
                }
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        if name.is_none() && topic.is_none() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[task_handler] requires at least one of: name = \"...\", topic = \"...\"",
            ));
        }

        Ok(TaskHandlerArgs { name, topic })
    }
}

/// Attribute macro to register an external task handler function with a name (activityId) and/or topic.
/// At least one of `name` or `topic` must be specified.
///
/// Usage in a binary or library depending on `operaton-task-worker`:
///
/// ```ignore
/// use operaton_task_worker_macros::task_handler;
/// use operaton_task_worker::types::{InputVariables, OutputVariables};
///
/// // Match by activity ID (task name):
/// #[task_handler(name = "example_echo")]
/// fn echo(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
///     Ok(std::collections::HashMap::new())
/// }
///
/// // Match by topic name:
/// #[task_handler(topic = "my-service-topic")]
/// fn topic_handler(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
///     Ok(std::collections::HashMap::new())
/// }
///
/// // Match by both name and topic:
/// #[task_handler(name = "ServiceTask_ID", topic = "my-service-topic")]
/// fn named_topic_handler(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
///     Ok(std::collections::HashMap::new())
/// }
/// ```
#[proc_macro_attribute]
pub fn task_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as TaskHandlerArgs);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_ident = input_fn.sig.ident.clone();

    let name_tokens = match &args.name {
        Some(n) => quote! { Some(#n) },
        None => quote! { None },
    };
    let topic_tokens = match &args.topic {
        Some(t) => quote! { Some(#t) },
        None => quote! { None },
    };

    // Resolve the runtime crate (operaton-task-worker) crate path as used by the depending crate
    let runtime_crate_ident = match crate_name("operaton-task-worker") {
        Ok(FoundCrate::Itself) => format_ident!("operaton_task_worker"),
        Ok(FoundCrate::Name(name)) => format_ident!("{}", name),
        Err(_) => format_ident!("operaton_task_worker"),
    };

    // Emit original function unchanged + inventory registration in the using crate's context
    let expanded = quote! {
        #input_fn

        const _: () = {
            // Ensure `inventory` is linked via the runtime crate and submit this handler
            #runtime_crate_ident::inventory::submit! {
                #runtime_crate_ident::registry::Handler::new(
                    #name_tokens,
                    #topic_tokens,
                    #fn_ident,
                )
            }
        };
    };

    TokenStream::from(expanded)
}
