#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, ItemTrait, parse_macro_input};

mod protocol;
mod rw;

#[proc_macro_derive(Read)]
pub fn derive_read(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(rw::derive_read(&input))
}

#[proc_macro_derive(Write)]
pub fn derive_write(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(rw::derive_write(&input))
}

/// Function-like macro for implementing `Read` and `Write` on already declared types.
///
/// Useful for implementing those traits on standard library types, like [`Result`] or [`Option`].
///
/// # Example
///
/// ```rs
/// ipc_macros::__impl_rw_for_external! {
///     enum Option<T> {
///         Some(T),
///         None
///     }
/// }
/// ```
#[doc(hidden)]
#[proc_macro]
pub fn __impl_rw_for_external(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let read_code = rw::derive_read(&input);
    let write_code = rw::derive_write(&input);
    TokenStream::from(quote! {
        #read_code
        #write_code
    })
}

#[proc_macro_attribute]
pub fn protocol(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as protocol::Arguments);
    let input = parse_macro_input!(input as ItemTrait);

    protocol::Protocol::parse(args, input).generate().into()
}
