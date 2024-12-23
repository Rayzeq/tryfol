use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemTrait};

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
pub fn protocol(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemTrait);
    TokenStream::from(protocol::protocol(input))
}
