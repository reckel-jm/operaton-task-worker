extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Meta, Expr, Lit};

/// Attribute macro to register an external task handler function with a name (activityId/topic).
/// Usage: `#[task_handler(name = "example_echo")]`
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

    // Emit original function unchanged + inventory registration in the using crate's context
    let expanded = quote! {
        #input_fn

        const _: () = {
            // Ensure `inventory` is linked and submit this handler
            inventory::submit! {
                crate::registry::Handler {
                    name: #name_value,
                    func: #fn_ident,
                }
            }
        };
    };

    TokenStream::from(expanded)
}
