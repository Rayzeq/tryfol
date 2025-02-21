use darling::{Error, FromMeta, ast::NestedMeta};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemTrait, parse_macro_input};

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

#[proc_macro_attribute]
pub fn protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(Error::from(e).write_errors());
        }
    };
    let args = match protocol::Arguments::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let input = parse_macro_input!(input as ItemTrait);
    TokenStream::from(protocol::Protocol::new(input, args).make())
}

#[proc_macro]
pub fn __impl_rw_for_external(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let read_code = rw::derive_read(input.clone());
    let write_code = rw::derive_write(input);
    TokenStream::from(quote! {
        #read_code
        #write_code
    })
}
