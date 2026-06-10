extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemFn, Meta, Expr, Lit};
use syn::parse::Parser;
use proc_macro_crate::{crate_name, FoundCrate};

#[proc_macro_attribute]
pub fn task_handler(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let attr2 = proc_macro2::TokenStream::from(attr);

    let mut name_value: Option<String> = None;
    let mut topic_value: Option<String> = None;

    if !attr2.is_empty() {
        // 1) Try to parse as a single string literal (shortcut for topic)
        if let Ok(lit_str) = syn::parse2::<syn::LitStr>(attr2.clone()) {
            topic_value = Some(lit_str.value());
        }
        // 2) Try to parse as Meta attributes (name = "...", topic = "...")
        else if let Ok(meta) = syn::parse2::<Meta>(attr2) {
            match meta {
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("name") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(s) = expr_lit.lit { name_value = Some(s.value()); }
                        }
                    } else if nv.path.is_ident("topic") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(s) = expr_lit.lit { topic_value = Some(s.value()); }
                        }
                    } else {
                        panic!("#[task_handler] unknown attribute key. Supported: name, topic");
                    }
                }
                Meta::List(list) => {
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
                _ => panic!("#[task_handler] requires syntax: #[task_handler(name = \"...\")] or #[task_handler(topic = \"...\")]")
            }
        } else {
            panic!("#[task_handler] invalid attribute format.");
        }
    }

    // Validation: Ensure at least one attribute is provided
    if name_value.is_none() && topic_value.is_none() {
        panic!("#[task_handler] requires at least one of 'name' or 'topic' (e.g. #[task_handler(\"my_topic\")] or #[task_handler(name = \"...\")]).");
    }

    // Map Option<String> directly to tokens representing Option<&'static str>
    // This allows the macro header to remain clean while satisfying the Handler struct
    let name_token = match name_value {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    let topic_token = match topic_value {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    let fn_ident = input_fn.sig.ident.clone();

    // Resolve the runtime crate (operaton-task-worker) crate path as used by the depending crate
    let runtime_crate_ident = match crate_name("operaton-task-worker") {
        Ok(FoundCrate::Itself) => format_ident!("operaton_task_worker"),
        Ok(FoundCrate::Name(name)) => format_ident!("{}", name),
        Err(_) => format_ident!("operaton_task_worker"),
    };

    // Emit original function unchanged + inventory registration matching the Option fields
    let expanded = quote! {
        #input_fn

        const _: () = {
            #runtime_crate_ident::inventory::submit! {
                #runtime_crate_ident::registry::Handler {
                    name: #name_token,
                    topic: #topic_token,
                    func: #fn_ident,
                }
            }
        };
    };

    TokenStream::from(expanded)
}