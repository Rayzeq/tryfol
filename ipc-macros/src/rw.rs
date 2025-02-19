use proc_macro2::TokenStream;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse_quote, spanned::Spanned, Data, DeriveInput, Fields, GenericParam, Generics, Ident, Index,
    Path, TypeParamBound, WherePredicate,
};

pub fn derive_read(input: DeriveInput) -> TokenStream {
    // Add a bound `T: Read` to every type parameter T.
    let (generics, additional_where_predicates) = add_trait_bounds(
        input.generics,
        &quote!(::ipc::rw::Read<Error: ::core::marker::Send + ::core::marker::Sync + 'static>),
        &parse_quote!(::ipc::rw::Read),
        matches!(input.data, Data::Enum(_)),
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
    where_clause.predicates.extend(additional_where_predicates);

    let read_code = read_code(&input.ident, &input.data);

    let name = input.ident;
    quote! {
        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics ::ipc::rw::Read for #name #ty_generics #where_clause {
            type Error = ::anyhow::Error;

            async fn read(stream: &mut (impl ::tokio::io::AsyncRead + ::core::marker::Unpin + ::core::marker::Send)) -> ::core::result::Result<Self, <Self as ::ipc::rw::Read>::Error>
            where
                Self: ::core::marker::Sized,
            {
                #read_code
            }
        }
    }
}

pub fn derive_write(input: DeriveInput) -> TokenStream {
    // Add a bound `T: Write` to every type parameter T.
    let (generics, additional_where_predicates) = add_trait_bounds(
        input.generics,
        &quote!(::ipc::rw::Write<Error: ::core::marker::Send + ::core::marker::Sync + 'static>),
        &parse_quote!(::ipc::rw::Write),
        false,
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
    where_clause.predicates.extend(additional_where_predicates);

    let write_code = write_code(&input.data);

    let name = input.ident;
    quote! {
        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics ::ipc::rw::Write for #name #ty_generics #where_clause {
            type Error = ::anyhow::Error;

            async fn write(
                &self,
                stream: &mut (impl ::tokio::io::AsyncWriteExt + ::core::marker::Unpin + ::core::marker::Send),
            ) -> ::core::result::Result<(), <Self as ::ipc::rw::Write>::Error>
            where
                Self: ::core::marker::Sized,
            {
                #write_code
            }
        }
    }
}

fn add_trait_bounds(
    mut generics: Generics,
    r#trait: &TokenStream,
    generic_trait: &TypeParamBound,
    is_enum_and_read: bool,
) -> (Generics, Vec<WherePredicate>) {
    let mut where_predicates = Vec::with_capacity(generics.params.len());
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(TypeParamBound::Verbatim(r#trait.clone()));
            type_param.bounds.push(parse_quote!(::core::marker::Sync));
            let name = &type_param.ident;
            where_predicates.push(
                parse_quote!(::anyhow::Error: ::core::convert::From<<#name as #generic_trait>::Error>),
            );
        }
    }

    if is_enum_and_read {
        where_predicates.push(
            parse_quote!(::anyhow::Error: ::core::convert::From<::ipc::rw::InvalidDiscriminantError>),
        );
    }

    (generics, where_predicates)
}

fn read_code(name: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            let constructor = read_code_for_fields(&parse_quote!(Self), &data.fields);
            quote! {
                ::core::result::Result::Ok(#constructor)
            }
        }
        Data::Enum(ref data) => {
            let recurse = data.variants.iter().enumerate().map(|(i, v)| {
                let name = &v.ident;
                let constructor = read_code_for_fields(&parse_quote!(Self::#name), &v.fields);
                quote_spanned! {v.span()=>
                    #i => ::core::result::Result::Ok(#constructor)
                }
            });
            let name = name.to_string();
            quote! {
                let discriminant = <u64 as ::ipc::rw::Read>::read(stream).await? as usize;
                match discriminant {
                    #(#recurse,)*
                    value => {
                        ::core::result::Result::Err(::anyhow::Error::from(::ipc::rw::InvalidDiscriminantError {
                            type_name: #name,
                            value,
                        }))
                    }
                }
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}

fn read_code_for_fields(this: &Path, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    #name: <#ty as ::ipc::rw::Read>::read(stream).await?
                }
            });
            quote! {
                #this {
                    #(#recurse,)*
                }
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().map(|f| {
                let ty = &f.ty;
                quote_spanned! {f.span()=>
                    <#ty as ::ipc::rw::Read>::read(stream).await?
                }
            });
            quote! {
                #this(#(#recurse,)*)
            }
        }
        Fields::Unit => this.to_token_stream(),
    }
}

fn write_code(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            let code = write_code_for_fields(&data.fields, true);
            quote! {
                #code;
                ::core::result::Result::Ok(())
            }
        }
        Data::Enum(ref data) => {
            if data.variants.is_empty() {
                return quote! {
                    ::std::unreachable!("Cannot write empty enum because there is no valid discriminant");
                };
            }
            let recurse = data.variants.iter().enumerate().map(|(i, v)| {
                let name = &v.ident;
                let fields = fields_to_pattern(&v.fields);
                let code = write_code_for_fields(&v.fields, false);
                quote_spanned! {v.span()=>
                    Self::#name #fields => {
                        ::ipc::rw::Write::write(&(#i as u64), stream).await?;
                        #code;
                    }
                }
            });
            quote! {
                #[allow(redundant_semicolons)]
                match self {
                    #(#recurse,)*
                }

                ::core::result::Result::Ok(())
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}

fn write_code_for_fields(fields: &Fields, has_self: bool) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;

                if has_self {
                    quote_spanned! {f.span()=>
                        ::ipc::rw::Write::write(&self.#name, stream).await?
                    }
                } else {
                    quote_spanned! {f.span()=>
                        ::ipc::rw::Write::write(#name, stream).await?
                    }
                }
            });
            quote! {
                #(#recurse;)*
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                if has_self {
                    let index = Index::from(i);
                    quote_spanned! {f.span()=>
                        ::ipc::rw::Write::write(&self.#index, stream).await?
                    }
                } else {
                    let name = Ident::new(&format!("f{i}"), f.span());
                    quote_spanned! {f.span()=>
                        ::ipc::rw::Write::write(#name, stream).await?
                    }
                }
            });
            quote! {
                #(#recurse;)*
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}

fn fields_to_pattern(fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let recurse = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap().clone());
            quote! {
                { #(#recurse,)* }
            }
        }
        Fields::Unnamed(ref fields) => {
            let recurse = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| Ident::new(&format!("f{i}"), f.span()));
            quote! {
                (#(#recurse,)*)
            }
        }
        Fields::Unit => TokenStream::new(),
    }
}
