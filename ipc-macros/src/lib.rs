use darling::{ast::NestedMeta, Error, FromMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, ItemTrait};

mod protocol;
mod rw;
mod utils;

#[proc_macro_derive(Read)]
pub fn derive_read(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(rw::derive_read(input))
}

#[proc_macro_derive(Write)]
pub fn derive_write(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(rw::derive_write(input))
}

#[derive(Default, FromMeta)]
#[darling(default)]
struct ProtocolArgs {
    #[darling(default)]
    abstract_socket: Option<String>,
}

#[proc_macro_attribute]
pub fn protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };
    let args = match ProtocolArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let input = parse_macro_input!(input as ItemTrait);
    TokenStream::from(protocol::protocol(args, input))
}

#[proc_macro]
pub fn __impl_rw_for_option(_input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_quote! {
        enum Option<T> {
            Some(T),
            None
        }
    };
    let read_code = rw::derive_read(input.clone());
    let write_code = rw::derive_write(input);
    TokenStream::from(quote! {
        #read_code
        #write_code
    })
}

#[proc_macro]
pub fn __impl_rw_for_result(_input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_quote! {
        enum Result<T, E> {
            Ok(T),
            Err(E)
        }
    };
    let read_code = rw::derive_read(input.clone());
    let write_code = rw::derive_write(input);
    TokenStream::from(quote! {
        #read_code
        #write_code
    })
}
