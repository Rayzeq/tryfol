use proc_macro::{Diagnostic, Level};
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    Expr, ExprLit, ExprPath, Ident, ItemTrait, Lit, Meta, MetaNameValue, Path, Result, Token,
    TraitItem, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

use super::{Arguments, Protocol, ProtocolMethod};

impl Parse for Arguments {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut abstract_socket = (None, Vec::new());
        let mut client_name = (None, Vec::new());
        let mut server_name = (None, Vec::new());

        let pairs = Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?;

        for pair in pairs {
            if pair.path.is_ident("abstract_socket") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(ref s),
                    ..
                }) = pair.value
                {
                    abstract_socket.0 = Some(s.value());
                } else {
                    Diagnostic::spanned(
                        pair.value.span().unwrap(),
                        Level::Error,
                        "abstract_socket must be a string literal",
                    )
                    .emit();
                }
                abstract_socket.1.push(pair);
            } else if pair.path.is_ident("client_name") {
                if let Expr::Path(ExprPath { ref path, .. }) = pair.value
                    && let Some(ident) = path.get_ident()
                {
                    client_name.0 = Some(ident.clone());
                } else {
                    Diagnostic::spanned(
                        pair.value.span().unwrap(),
                        Level::Error,
                        "client_name must be an identifier",
                    )
                    .emit();
                }
                client_name.1.push(pair);
            } else if pair.path.is_ident("server_name") {
                if let Expr::Path(ExprPath { ref path, .. }) = pair.value
                    && let Some(ident) = path.get_ident()
                {
                    server_name.0 = Some(ident.clone());
                } else {
                    Diagnostic::spanned(
                        pair.value.span().unwrap(),
                        Level::Error,
                        "server_name must be an identifier",
                    )
                    .emit();
                }
                server_name.1.push(pair);
            } else {
                Diagnostic::spanned(
                    pair.path.span().unwrap(),
                    Level::Error,
                    format!("unknown argument: {}", pair.path.to_token_stream()),
                )
                .emit();
            }
        }

        emit_duplicate_warnings(&mut abstract_socket.1, "abstract_socket");
        emit_duplicate_warnings(&mut client_name.1, "client_name");
        emit_duplicate_warnings(&mut server_name.1, "server_name");

        Ok(Self {
            abstract_socket: abstract_socket.0,
            client_name: client_name.0,
            server_name: server_name.0,
        })
    }
}

impl Protocol {
    pub fn parse(args: Arguments, input: ItemTrait) -> Self {
        let name = &input.ident;
        let module_name = Ident::new(&format!("__{name}_inner"), Span::mixed_site());
        let client_name = args
            .client_name
            .unwrap_or_else(|| Ident::new(&(name.to_string() + "Client"), Span::mixed_site()));
        let server_name = args
            .server_name
            .unwrap_or_else(|| Ident::new(&(name.to_string() + "Server"), Span::mixed_site()));

        if let Some(unsafety) = input.unsafety {
            Diagnostic::spanned(
                unsafety.span().unwrap(),
                Level::Error,
                "protocol trait cannot be unsafe",
            )
            .emit();
        }

        if let Some(auto_token) = input.auto_token {
            Diagnostic::spanned(
                auto_token.span().unwrap(),
                Level::Error,
                "protocol trait cannot an auto trait",
            )
            .emit();
        }

        Self {
            abstract_socket: args.abstract_socket,
            module_name,
            client_name,
            server_name,
            attributes: input.attrs,
            visibility: input.vis,
            name: input.ident,
            generics: input.generics,
            supertraits: input.supertraits,
            methods: input
                .items
                .into_iter()
                .filter_map(ProtocolMethod::parse)
                .collect(),
        }
    }
}

impl ProtocolMethod {
    fn parse(input: TraitItem) -> Option<Self> {
        match input {
            TraitItem::Fn(mut item_fn) => {
                if item_fn.sig.asyncness.is_none() {
                    Diagnostic::spanned(
                        item_fn.sig.span().unwrap(),
                        Level::Error,
                        "protocol method must be async",
                    )
                    .emit();
                }

                let attribute = item_fn
                    .attrs
                    .extract_if(.., |x| x.path().is_ident("stream"))
                    // TODO: emit diagnostic if there's multiple `stream` parameter instead of just ignoring them
                    .next();
                Some(if let Some(attribute) = attribute {
                    if matches!(attribute.meta, Meta::Path(_)) {
                        Self::LongCall {
                            method: item_fn,
                            early_error: None,
                        }
                    } else {
                        let pairs = match attribute.parse_args_with(
                            Punctuated::<MetaNameType, Token![,]>::parse_terminated,
                        ) {
                            Ok(x) => x.into_iter().collect(),
                            Err(e) => {
                                e.span().unwrap().error(e.to_string()).emit();
                                Vec::new()
                            }
                        };

                        let mut early_error = (None, Vec::new());
                        for pair in pairs {
                            if pair.path.is_ident("early_error") {
                                early_error.0 = Some(pair.r#type.clone());
                                early_error.1.push(pair);
                            } else {
                                Diagnostic::spanned(
                                    pair.path.span().unwrap(),
                                    Level::Error,
                                    format!("unknown argument: {}", pair.path.to_token_stream()),
                                )
                                .emit();
                            }
                        }

                        emit_duplicate_warnings(&mut early_error.1, "early_error");

                        Self::LongCall {
                            method: item_fn,
                            early_error: early_error.0,
                        }
                    }
                } else {
                    Self::SimpleCall(item_fn)
                })
            }
            item => {
                Diagnostic::spanned(
                    item.span().unwrap(),
                    Level::Error,
                    "unsupported item in protocol",
                )
                .note("consider moving this item to a new supertrait")
                .emit();
                None
            }
        }
    }
}

struct MetaNameType {
    pub path: Path,
    pub eq_token: Token![=],
    pub r#type: Type,
}

impl Parse for MetaNameType {
    fn parse(input: ParseStream) -> Result<Self> {
        let path = Path::parse_mod_style(input)?;
        let eq_token = input.parse()?;
        let r#type = input.parse()?;

        Ok(Self {
            path,
            eq_token,
            r#type,
        })
    }
}

impl ToTokens for MetaNameType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.path.to_tokens(tokens);
        self.eq_token.to_tokens(tokens);
        self.r#type.to_tokens(tokens);
    }
}

fn emit_duplicate_warnings<T: Spanned>(pairs: &mut Vec<T>, name: &'static str) {
    if let Some(last_pair) = pairs.pop()
        && !pairs.is_empty()
    {
        let mut diagnostic = Diagnostic::spanned(
            last_pair.span().unwrap(),
            Level::Warning,
            format!(
                "multiple declaration of `{name}`, the latest declaration will shadow previous ones"
            ),
        );
        for pair in pairs {
            diagnostic = diagnostic.span_note(pair.span().unwrap(), "previous declaration here");
        }
        diagnostic.emit();
    }
}
