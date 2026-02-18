use syn::{Attribute, Generics, Ident, Token, TraitItemFn, Type, TypeParamBound, Visibility, punctuated::Punctuated};

mod generate;
mod parse;

pub struct Arguments {
    abstract_socket: Option<String>,
    client_name: Option<Ident>,
    server_name: Option<Ident>,
}

pub struct Protocol {
    abstract_socket: Option<String>,
    module_name: Ident,
    client_name: Ident,
    server_name: Ident,

    attributes: Vec<Attribute>,
    visibility: Visibility,
    name: Ident,
    generics: Generics,
    supertraits: Punctuated<TypeParamBound, Token![+]>,
    methods: Vec<ProtocolMethod>,
}

#[expect(clippy::large_enum_variant, reason = "not a huge difference and enum won't have a lot of instances")]
#[derive(Clone)]
enum ProtocolMethod {
    SimpleCall(TraitItemFn),
    LongCall { method: TraitItemFn, early_error: Option<Type> },
}

impl ProtocolMethod {
    pub const fn inner(&self) -> &TraitItemFn {
        match self {
            Self::SimpleCall(method) | Self::LongCall { method, .. } => method,
        }
    }

    pub const fn inner_mut(&mut self) -> &mut TraitItemFn {
        match self {
            Self::SimpleCall(method) | Self::LongCall { method, .. } => method,
        }
    }
}
