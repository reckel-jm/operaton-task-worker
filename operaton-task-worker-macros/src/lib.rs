/*
The crate [operaton_task_worker_macros] provides a proc-macro attribute macro to register an external task handler function with a name (activityId/topic).
*/

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, Meta, Expr, Lit};
use syn::parse::Parser;
use proc_macro_crate::{crate_name, FoundCrate};

/// Attribute macro to register an external task handler function with a name (activityId/topic).
/// Usage in a binary or library depending on `operaton-task-worker`:
///
/// ```ignore
/// use operaton_task_worker_macros::task_handler;
/// use operaton_task_worker::types::{InputVariables, OutputVariables};
///
/// #[task_handler(name = "example_echo")]
/// fn echo(_input: &InputVariables) -> Result<OutputVariables, Box<dyn std::error::Error>> {
///     Ok(std::collections::HashMap::new())
/// }
/// ```
#[proc_macro_attribute]
pub fn task_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Accept meta like: name = "...", optional topic = "..."
    let meta = parse_macro_input!(attr as Meta);
    let input_fn = parse_macro_input!(item as ItemFn);

    // Defaults
    let mut name_value: Option<String> = None;
    let mut topic_value: Option<String> = None;

    match meta {
        Meta::NameValue(nv) => {
            // Single `name = "..."` or `topic = "..."`
            if nv.path.is_ident("name") {
                if let Expr::Lit(expr_lit) = nv.value {
                    if let Lit::Str(s) = expr_lit.lit { name_value = Some(s.value()); } else { panic!("#[task_handler] expects name as string literal") }
                } else { panic!("#[task_handler] expects name as string literal") }
            } else if nv.path.is_ident("topic") {
                if let Expr::Lit(expr_lit) = nv.value {
                    if let Lit::Str(s) = expr_lit.lit { topic_value = Some(s.value()); } else { panic!("#[task_handler] expects topic as string literal") }
                } else { panic!("#[task_handler] expects topic as string literal") }
            } else {
                panic!("#[task_handler] unknown attribute key. Supported: name, topic");
            }
        }
        Meta::List(list) => {
            // Parse list tokens into comma separated MetaNameValue items
            let parser = syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated;
            let pairs = parser.parse2(list.tokens).expect("#[task_handler] invalid attribute list. Expected name-value pairs.");
            for nv in pairs {
                if nv.path.is_ident("name") {
                    if let Expr::Lit(expr_lit) = nv.value.clone() {
                        if let Lit::Str(s) = expr_lit.lit { name_value = Some(s.value()); } else { panic!("#[task_handler] expects name as string literal") }
                    } else { panic!("#[task_handler] expects name as string literal") }
                } else if nv.path.is_ident("topic") {
                    if let Expr::Lit(expr_lit) = nv.value.clone() {
                        if let Lit::Str(s) = expr_lit.lit { topic_value = Some(s.value()); } else { panic!("#[task_handler] expects topic as string literal") }
                    } else { panic!("#[task_handler] expects topic as string literal") }
                } else {
                    panic!("#[task_handler] unknown attribute key. Supported: name, topic");
                }
            }
        }
        _ => panic!("#[task_handler] requires syntax: #[task_handler(name = \"...\")] or with optional topic = \"...\"")
    }

    let name_value = name_value.expect("#[task_handler] missing required 'name' attribute");

    let fn_ident = input_fn.sig.ident.clone();

    // Resolve the runtime crate (operaton-task-worker) crate path as used by the depending crate
    let runtime_crate_ident = match crate_name("operaton-task-worker") {
        Ok(FoundCrate::Itself) => format_ident!("operaton_task_worker"),
        Ok(FoundCrate::Name(name)) => format_ident!("{}", name),
        Err(_) => format_ident!("operaton_task_worker"),
    };

    // Emit original function unchanged + inventory registration in the using crate's context
    let expanded = if let Some(topic_literal) = topic_value {
        quote! {
            #input_fn

            const _: () = {
                // Ensure `inventory` is linked via the runtime crate and submit this handler
                #runtime_crate_ident::inventory::submit! {
                    #runtime_crate_ident::registry::Handler {
                        name: #name_value,
                        topic: Some(#topic_literal),
                        func: #fn_ident,
                    }
                }
            };
        }
    } else {
        quote! {
            #input_fn

            const _: () = {
                // Ensure `inventory` is linked via the runtime crate and submit this handler
                #runtime_crate_ident::inventory::submit! {
                    #runtime_crate_ident::registry::Handler {
                        name: #name_value,
                        topic: None,
                        func: #fn_ident,
                    }
                }
            };
        }
    };

    TokenStream::from(expanded)
}
