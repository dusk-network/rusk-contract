#![feature(external_doc)]
#![deny(missing_docs)]
#![doc(include = "../README.md")]

extern crate proc_macro;
use lazy_static::lazy_static;
use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static! {
    static ref OPCODES: Mutex<HashMap<u8, (String, bool)>> =
        Mutex::new(HashMap::new());

        // TODO: try maybe to use `AtomicBool` instead
    static ref MAIN_CALLED: Mutex<bool> = Mutex::new(false);
}

#[proc_macro_attribute]
/// A macro for declaring methods callable for a Rusk Contract.
/// It needs to have an `opcode` specified.
pub fn method(attr: TokenStream, item: TokenStream) -> TokenStream {
    let already_called = MAIN_CALLED.lock().unwrap();
    assert!(
        !(*already_called),
        "::main already called; methods need to be declared first."
    );

    let attrs = syn::parse_macro_input!(attr as syn::AttributeArgs);
    assert!(attrs.len() == 1, "only one attribute can be defined");

    let argument_name_and_value = match attrs.get(0) {
        Some(syn::NestedMeta::Meta(syn::Meta::NameValue(meta))) => meta,
        _ => panic!("expected argument `opcode = <u8>`"),
    };
    let path = argument_name_and_value.path.get_ident().unwrap();
    assert!(
        *path == "opcode",
        format!(
            "Only opcode attribute can be set (found \"{}\")",
            path.to_string()
        )
    );

    let opcode = match &argument_name_and_value.lit {
        syn::Lit::Int(lit) => lit.base10_parse::<u8>().unwrap(),
        _ => panic!("expected argument value to be a u8"),
    };

    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let name = &input.sig.ident;
    let ret = &input.sig.output;
    let body = &input.block;
    let vis = &input.vis;
    let inputs = &input.sig.inputs;

    let struct_args: Vec<syn::Ident> = inputs
        .iter()
        .filter_map(|i| match i {
            syn::FnArg::Typed(t) => match *(t.pat.clone()) {
                syn::Pat::Ident(ident) => {
                    if ident.ident == "self" {
                        None
                    } else {
                        Some(ident.ident)
                    }
                }
                _ => panic!(
                    "You have to use simple identifiers for delegated method parameters ({})",
                    input.sig.ident
                ),
            },
            _ => None,
        })
        .collect();

    let mut hm = OPCODES.lock().unwrap();

    assert!(
        !hm.contains_key(&opcode),
        format!(
            "opcodes needs to be unique per contract (`{}` already exists)",
            opcode
        ),
    );

    let struct_name = syn::Ident::new(&format!("{}_args", name), proc_macro2::Span::call_site());

    let struct_types: Vec<syn::Type> = inputs
        .iter()
        .filter_map(|i| match i {
            syn::FnArg::Typed(t) => Some(*t.ty.clone()),
            _ => None,
        })
        .collect();
    let result = if struct_types.is_empty() {
        quote! {
            #vis fn #name() #ret {
                #body
             }
        }
    } else {
        quote! {
            #[repr(C, packed)]
            pub struct #struct_name (
                #(#struct_types,)*
            );
            unsafe impl Pod for #struct_name {}

            #vis fn #name(#struct_name(#(#struct_args,)*): #struct_name) #ret {
                #body
             }
        }
    };

    hm.insert(opcode, (name.to_string(), !struct_types.is_empty()));

    result.into()
}

#[proc_macro_attribute]
/// A macro for declaring the main entry point for a Rusk Contract.
/// It needs to be specified after all the contract's methods.
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut already_called = MAIN_CALLED.lock().unwrap();
    assert_eq!(*already_called, false);
    *already_called = true;

    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let ret = &input.sig.output;
    let body = &input.block;
    let stmts = &body.stmts;

    let hm = OPCODES.lock().unwrap().clone();
    let keys = hm.keys();

    let values: Vec<syn::ExprCall> = hm
        .values()
        .map(|(k, a)| {
            let k = syn::Ident::new(k, proc_macro2::Span::call_site());
            if *a {
                syn::parse_quote! { #k(dusk_abi::argument()) }
            } else {
                syn::parse_quote! { #k() }
            }
        })
        .collect();

    let ty: syn::Type = match ret {
        syn::ReturnType::Type(_, ty) => *ty.clone(),
        syn::ReturnType::Default => syn::parse_quote! { i32 },
    };

    let result = if keys.len() > 0 {
        quote! {
            #[no_mangle]
            pub fn call() #ret {
                #body
                let code: u8 = dusk_abi::opcode::<u8>();
                dusk_abi::ret::<#ty>(match code {
                    #( #keys => #values,) *
                    _ => <#ty>::default(),
                });
            }
        }
    } else {
        quote! {
            #[no_mangle]
            pub fn call() #ret {
                #(#stmts),*
            }
        }
    };
    result.into()
}
