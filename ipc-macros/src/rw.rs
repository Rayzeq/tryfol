use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Fields, GenericParam, Generics, Ident, Index, Path, TypeParamBound,
    WherePredicate, parse_quote, spanned::Spanned,
};

pub fn derive_read(input: &DeriveInput) -> TokenStream {
    // Add a bound `T: Read` to every type parameter T.
    let (generics, additional_where_predicates) = add_trait_bounds(
        input.generics.clone(),
        &quote!(::ipc::Read<Error: ::core::marker::Send + ::core::marker::Sync + 'static>),
        &parse_quote!(::ipc::Read),
        true,
        matches!(input.data, Data::Enum(_)),
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
    where_clause.predicates.extend(additional_where_predicates);

    let read_code = read_code(&input.ident, &input.data);

    let name = &input.ident;
    quote! {
        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics ::ipc::Read for #name #ty_generics #where_clause {
            type Error = ::ipc::anyhow::Error;

            async fn read(stream: &mut (impl ::ipc::tokio::io::AsyncRead + ::core::marker::Unpin + ::core::marker::Send)) -> ::core::result::Result<Self, <Self as ::ipc::Read>::Error>
            where
                Self: ::core::marker::Sized,
            {
                #read_code
            }
        }
    }
}

pub fn derive_write(input: &DeriveInput) -> TokenStream {
    // Add a bound `T: Write` to every type parameter T.
    let (generics, additional_where_predicates) = add_trait_bounds(
        input.generics.clone(),
        &quote!(::ipc::Write<Error: ::core::marker::Send + ::core::marker::Sync + 'static>),
        &parse_quote!(::ipc::Write),
        false,
        matches!(input.data, Data::Enum(_)),
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| parse_quote!(where));
    where_clause.predicates.extend(additional_where_predicates);

    let write_code = write_code(&input.data);

    let name = &input.ident;
    quote! {
        #[allow(clippy::trait_duplication_in_bounds)]
        impl #impl_generics ::ipc::Write for #name #ty_generics #where_clause {
            type Error = ::ipc::anyhow::Error;

            async fn write(
                &self,
                stream: &mut (impl ::ipc::tokio::io::AsyncWriteExt + ::core::marker::Unpin + ::core::marker::Send),
            ) -> ::core::result::Result<(), <Self as ::ipc::Write>::Error>
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
    is_read: bool,
    is_enum: bool,
) -> (Generics, Vec<WherePredicate>) {
    let mut where_predicates = Vec::with_capacity(generics.params.len());
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(TypeParamBound::Verbatim(r#trait.clone()));
            type_param.bounds.push(parse_quote!(::core::marker::Sync));
            if is_read {
                type_param.bounds.push(parse_quote!(::core::marker::Send));
            }

            let name = &type_param.ident;
            where_predicates.push(parse_quote!(::ipc::anyhow::Error: ::core::convert::From<<#name as #generic_trait>::Error>));
        }
    }

    if is_read && is_enum {
        where_predicates.push(parse_quote!(::ipc::anyhow::Error: ::core::convert::From<::ipc::InvalidDiscriminantError>));
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
                quote! {
                    #i => ::core::result::Result::Ok(#constructor)
                }
            });
            let name = name.to_string();
            quote! {
                let discriminant = <u64 as ::ipc::Read>::read(stream).await? as usize;
                match discriminant {
                    #(#recurse,)*
                    value => {
                        ::core::result::Result::Err(::ipc::anyhow::Error::from(::ipc::InvalidDiscriminantError {
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
        Fields::Named(fields) => {
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;
                let ty = &f.ty;
                quote! {
                    #name: <#ty as ::ipc::Read>::read(stream).await?
                }
            });
            quote! {
                #this {
                    #(#recurse,)*
                }
            }
        }
        Fields::Unnamed(fields) => {
            let recurse = fields.unnamed.iter().map(|f| {
                let ty = &f.ty;
                quote! {
                    <#ty as ::ipc::Read>::read(stream).await?
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
    match data {
        Data::Struct(data) => {
            let code = write_code_for_fields(&data.fields, true);
            quote! {
                #code;
                ::core::result::Result::Ok(())
            }
        }
        Data::Enum(data) => {
            if data.variants.is_empty() {
                return quote! {
                    ::core::unreachable!("Cannot write empty enum because there is no valid discriminant");
                };
            }
            let recurse = data.variants.iter().enumerate().map(|(i, v)| {
                let name = &v.ident;
                let fields = fields_to_pattern(&v.fields);
                let code = write_code_for_fields(&v.fields, false);
                quote! {
                    Self::#name #fields => {
                        ::ipc::Write::write(&(#i as u64), stream).await?;
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
        Fields::Named(fields) => {
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;

                if has_self {
                    quote! {
                        ::ipc::Write::write(&self.#name, stream).await?
                    }
                } else {
                    quote! {
                        ::ipc::Write::write(#name, stream).await?
                    }
                }
            });
            quote! {
                #(#recurse;)*
            }
        }
        Fields::Unnamed(fields) => {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                if has_self {
                    let index = Index::from(i);
                    quote! {
                        ::ipc::Write::write(&self.#index, stream).await?
                    }
                } else {
                    let name = Ident::new(&format!("f{i}"), f.span());
                    quote! {
                        ::ipc::Write::write(#name, stream).await?
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
        Fields::Named(fields) => {
            let recurse = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().unwrap().clone());
            quote! {
                { #(#recurse,)* }
            }
        }
        Fields::Unnamed(fields) => {
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
