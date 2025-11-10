extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, Meta, Expr, Lit};
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
    // Accept a single name-value meta: name = "..."
    let meta = parse_macro_input!(attr as Meta);
    let input_fn = parse_macro_input!(item as ItemFn);

    let name_value = match meta {
        Meta::NameValue(nv) if nv.path.is_ident("name") => match nv.value {
            Expr::Lit(expr_lit) => match expr_lit.lit {
                Lit::Str(s) => s.value(),
                _ => panic!("#[task_handler] expects name to be a string literal: name = \"...\""),
            },
            _ => panic!("#[task_handler] expects name to be a string literal: name = \"...\""),
        },
        _ => panic!("#[task_handler] requires syntax: #[task_handler(name = \"...\")]"),
    };

    let fn_ident = input_fn.sig.ident.clone();

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
                #runtime_crate_ident::registry::Handler {
                    name: #name_value,
                    func: #fn_ident,
                }
            }
        };
    };

    TokenStream::from(expanded)
}
