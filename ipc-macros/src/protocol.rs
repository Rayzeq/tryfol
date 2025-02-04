use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, FnArg, Ident, ItemTrait, TraitItemFn, Type};

use crate::{
    utils::{FunctionEditor, ParseType, TraitEditor},
    ProtocolArgs,
};

pub fn protocol(args: ProtocolArgs, input: ItemTrait) -> TokenStream {
    let modname = Ident::new(&(input.ident.to_string() + "_inner"), input.ident.span());
    let inner_module = inner_module::make(&input, &modname);
    let client = make_client(args, &input, &modname);

    let client_trait = make_client_trait(input.clone(), &modname);
    let server_trait = server_trait::make(input, &modname);
    quote! {
        #inner_module
        #client_trait
        #client
        #server_trait
    }
}

fn is_stream(ty: &impl ParseType) -> bool {
    ty.as_result()
        .map_or(false, |result| result.ok().as_stream().is_some())
}

fn get_response_type(method: &TraitItemFn) -> Type {
    let return_type = method
        .sig
        .output
        .as_result()
        .map_or_else(|| parse_quote!(()), |ty| ty.ok().clone());

    return_type
        .as_stream()
        .and_then(|stream| stream.item().as_result().map(|result| result.ok().clone()))
        .unwrap_or(return_type)
}

mod inner_module {
    use crate::{
        protocol::{get_response_type, is_stream},
        utils::TraitEditor,
    };
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{parse_quote, FnArg, Ident, ItemMod, ItemTrait};

    pub fn make(input: &ItemTrait, module_name: &Ident) -> ItemMod {
        let response = make_response(input);
        let response_structs = make_response_structs(input);
        let method_call = make_method_call(input);
        let call_structs = make_call_structs(input);

        parse_quote! {
            #[allow(non_snake_case, non_camel_case_types)]
            mod #module_name {
                use super::*;

                #response
                #(#response_structs)*
                #method_call
                #(#call_structs)*
            }
        }
    }

    fn make_response(input: &ItemTrait) -> TokenStream {
        let variants: Vec<_> = input
            .methods()
            .map(|method| {
                let name = &method.sig.ident;
                let struct_name = Ident::new(&(name.to_string() + "Response"), name.span());
                quote! {
                    #name(#struct_name)
                }
            })
            .collect();

        quote! {
            #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
            pub enum Response {
                #(#variants,)*
                EndOfStream,
                Error(String),
            }

            impl ::ipc::AnyResponse for Response {}
        }
    }

    fn make_response_structs(input: &ItemTrait) -> Vec<TokenStream> {
        input
            .methods()
            .map(|method| {
                let name = &method.sig.ident;
                let field_type = get_response_type(method);
                let struct_name = Ident::new(&(name.to_string() + "Response"), name.span());

                let conditional = if is_stream(&method.sig.output) {
                    quote! {
                        impl ::core::convert::TryFrom<Response> for ::core::option::Option<#struct_name> {
                            type Error = ::ipc::ClientError<Response>;

                            fn try_from(value: Response) -> ::core::result::Result<Self, Self::Error> {
                                match value {
                                    Response::#name(response) => ::core::result::Result::Ok(::core::option::Option::Some(response)),
                                    Response::EndOfStream => ::core::result::Result::Ok(::core::option::Option::None),
                                    Response::Error(error) => ::core::result::Result::Err(::ipc::ClientError::Server(error)),
                                    value => ::core::result::Result::Err(::ipc::ClientError::Type(value)),
                                }
                            }
                        }
                    }
                } else {
                    quote! {
                        impl ::core::convert::TryFrom<Response> for #struct_name {
                            type Error = ::ipc::ClientError<Response>;

                            fn try_from(value: Response) -> ::core::result::Result<Self, Self::Error> {
                                match value {
                                    Response::#name(response) => ::core::result::Result::Ok(response),
                                    Response::Error(error) => ::core::result::Result::Err(::ipc::ClientError::Server(error)),
                                    value => ::core::result::Result::Err(::ipc::ClientError::Type(value)),
                                }
                            }
                        }
                    }
                };


                quote! {
                    #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
                    pub struct #struct_name(pub #field_type);

                    impl ::ipc::Response for #struct_name {
                        type Inner = #field_type;

                        fn into_inner(self) -> Self::Inner {
                            self.0
                        }
                    }

                    #conditional
                }
            })
            .collect()
    }

    fn make_method_call(input: &ItemTrait) -> TokenStream {
        let variants: Vec<_> = input
            .methods()
            .map(|method| {
                let name = &method.sig.ident;
                let struct_name = Ident::new(&(name.to_string() + "Call"), name.span());

                quote! {
                    #name(#struct_name)
                }
            })
            .collect();

        quote! {
            #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
            pub enum MethodCall {
                #(#variants,)*
            }

            impl ::ipc::AnyCall for MethodCall {
                type Response = Response;
            }
        }
    }

    fn make_call_structs(input: &ItemTrait) -> Vec<TokenStream> {
        input
            .methods()
            .map(|method| {
                let variant_name = &method.sig.ident;
                let name = Ident::new(&(variant_name.to_string() + "Call"), variant_name.span());
                let response_name = Ident::new(
                    &(variant_name.to_string() + "Response"),
                    variant_name.span(),
                );

                let fields = method.sig.inputs.iter().filter_map(|arg| {
                    if let FnArg::Typed(arg) = arg {
                        let name = &arg.pat;
                        let ty = &arg.ty;
                        Some(quote! { pub #name: #ty })
                    } else {
                        None
                    }
                });

                let conditional = if is_stream(&method.sig.output) {
                    quote! {
                        impl ::ipc::LongMethod for #name {
                            type Response = #response_name;
                        }
                    }
                } else {
                    quote! {
                        impl ::ipc::Method for #name {
                            type Response = #response_name;
                        }
                    }
                };

                quote! {
                    #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
                    pub struct #name {
                        #(#fields,)*
                    }

                    impl ::core::convert::From<#name> for MethodCall {
                        fn from(value: #name) -> Self {
                            Self::#variant_name(value)
                        }
                    }

                    #conditional
                }
            })
            .collect()
    }
}

fn make_client_trait(mut input: ItemTrait, module_name: &Ident) -> ItemTrait {
    input.methods_mut().for_each(|method| {
        if let Some(mut result) = method.sig.output.as_result_mut() {
            if let Some(mut stream) = result.ok_mut().as_stream_mut() {
                if let Some(mut result) = stream.item_mut().as_result_mut() {
                    if result.err().is_none() {
                        result.set_err(parse_quote!(#module_name::Response));
                    }
                }
            }
            if result.err().is_none() {
                result.set_err(parse_quote!(#module_name::Response));
            }
        }

        method.add_async_send_bound();
    });

    input
}

fn make_client(args: ProtocolArgs, input: &ItemTrait, module_name: &Ident) -> TokenStream {
    let vis = &input.vis;
    let trait_name = &input.ident;
    let client_name = Ident::new(&(trait_name.to_string() + "Client"), trait_name.span());

    let methods: Vec<_> = input
        .methods()
        .map(|method| {
            let name = &method.sig.ident;
            let args = &method.sig.inputs;

            let call_name = Ident::new(&(name.to_string() + "Call"), name.span());
            let return_type = get_response_type(method);
            let construct: Vec<_> = method.sig.inputs.iter().filter_map(|arg| {
                if let FnArg::Typed(arg) = arg {
                    Some(arg.pat.clone())
                } else {
                    None
                }
            }).collect();

            if is_stream(&method.sig.output) {
                quote! {
                    async fn #name(#args) -> ::ipc::Result<impl ::ipc::futures::Stream<Item = ::ipc::Result<#return_type, #module_name::Response>>, #module_name::Response>
                    {
                        ::ipc::Connection::long_call(&self.inner, #module_name::#call_name { #(#construct,)* }).await
                    }
                }
            } else {
                quote! {
                    async fn #name(#args) -> ::ipc::Result<#return_type, #module_name::Response> {
                        ::ipc::Connection::call(&self.inner, #module_name::#call_name { #(#construct,)* }).await
                    }
                }
            }
        })
        .collect();

    let mut additional_impls = Vec::new();
    if let Some(socket_name) = args.abstract_socket {
        additional_impls.push(quote! {
            impl #client_name<::tokio::net::unix::OwnedWriteHalf> {
                #vis fn new() -> ::std::io::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        inner: ::ipc::Connection::from_unix_address(&<::std::os::unix::net::SocketAddr as ::std::os::linux::net::SocketAddrExt>::from_abstract_name(#socket_name)?)?,
                    })
                }
            }
        });
    }

    quote! {
        #[derive(::core::fmt::Debug)]
        #[allow(clippy::module_name_repetitions)]
        #vis struct #client_name<T: ::ipc::tokio::io::AsyncWriteExt + ::core::marker::Unpin + ::core::marker::Send> {
            inner: ::ipc::Connection<#module_name::MethodCall, T>,
        }

        impl<T: ::ipc::tokio::io::AsyncWriteExt + ::core::marker::Unpin + ::core::marker::Send> #trait_name for #client_name<T> {
            #(#methods)*
        }

        #(#additional_impls)*
    }
}

mod server_trait {
    use crate::{
        protocol::is_stream,
        utils::{FunctionEditor, ParseType, TraitEditor},
    };
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::{parse_quote, FnArg, Ident, ItemTrait, TraitItem};

    pub fn make(mut input: ItemTrait, module_name: &Ident) -> ItemTrait {
        input.set_name(Ident::new(
            &(input.ident.to_string() + "Server"),
            input.ident.span(),
        ));
        input
            .attrs
            .push(parse_quote!(#[allow(clippy::module_name_repetitions)]));

        let handle_client_method = make_handle_client_method(&input, module_name);
        input.items.push(handle_client_method);

        input.methods_mut().for_each(|method| {
            if let Some(mut result) = method.sig.output.as_result_mut() {
                if let Some(mut stream) = result.ok_mut().as_stream_mut() {
                    if let Some(mut result) = stream.item_mut().as_result_mut() {
                        if result.err().is_none() {
                            result.set_err(parse_quote!(
                                impl ::core::fmt::Display + ::core::marker::Send
                            ));
                        }
                        result.set_path(parse_quote!(::core::result));
                    }
                }
                if result.err().is_none() {
                    result.set_err(parse_quote!(
                        impl ::core::fmt::Display + ::core::marker::Send
                    ));
                }
                result.set_path(parse_quote!(::core::result));
            }

            method.add_async_send_bound();
        });

        input
    }

    fn make_handle_client_method(input: &ItemTrait, module_name: &Ident) -> TraitItem {
        let branches = make_packet_branches(input, module_name);

        parse_quote! {
            fn handle_client(
                self,
                mut rx: impl ::ipc::tokio::io::AsyncReadExt + ::core::marker::Unpin + ::core::marker::Send,
                tx: impl ::ipc::tokio::io::AsyncWriteExt + ::core::marker::Unpin + ::core::marker::Send + 'static,
            ) -> impl ::core::future::Future<Output = ()> + ::core::marker::Send
            where
                Self: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static,
            {
                async move {
                    struct CancelGuard {
                        tasks: ::std::vec::Vec<::ipc::tokio::task::JoinHandle<()>>,
                    }

                    impl ::core::ops::Drop for CancelGuard {
                        fn drop(&mut self) {
                            for task in &self.tasks {
                                ::ipc::tokio::task::JoinHandle::abort(&task);
                            }
                        }
                    }

                    let tx = ::std::sync::Arc::new(::ipc::tokio::sync::Mutex::new(tx));
                    let mut cancel_guard = CancelGuard {
                        tasks: ::std::vec::Vec::new(),
                    };
                    loop {
                        match <::ipc::packet::Clientbound::<#module_name::MethodCall> as ::ipc::rw::Read>::read(&mut rx).await {
                            ::core::result::Result::Ok(packet) => {
                                let tx = <::std::sync::Arc<_> as ::core::clone::Clone>::clone(&tx);
                                let this = <Self as ::core::clone::Clone>::clone(&self);
                                ::std::vec::Vec::push(&mut cancel_guard.tasks, ::ipc::tokio::spawn(async move {
                                    macro_rules! send_packet {
                                        ($payload:expr) => {
                                            let packet = ::ipc::packet::Serverbound {
                                                call_id: packet.call_id,
                                                payload: $payload,
                                            };
                                            let result = <::ipc::packet::Serverbound<_> as ::ipc::rw::Write>::write(&packet, &mut *::tokio::sync::Mutex::lock(&tx).await).await;
                                            if let ::core::result::Result::Err(e) = result {
                                                if let ::core::option::Option::Some(e) = ::ipc::anyhow::Error::downcast_ref::<::std::io::Error>(&e) {
                                                    if ::std::io::Error::kind(&e) == ::std::io::ErrorKind::BrokenPipe {
                                                        // client quitted normally
                                                        return;
                                                    }
                                                }
                                                ::ipc::log::error!("Error while sending packet to client: {e:?}");
                                            }
                                        };
                                    }

                                    match packet.payload {
                                        #(#branches)*
                                    }
                                }));
                            }
                            ::core::result::Result::Err(e) => {
                                if let ::core::option::Option::Some(e) = ::ipc::anyhow::Error::downcast_ref::<::std::io::Error>(&e) {
                                    if ::std::io::Error::kind(&e) == ::std::io::ErrorKind::UnexpectedEof {
                                        // client quitted normally
                                        break;
                                    }
                                }
                                ::ipc::log::error!("Error receiving message from client: {e:?}");
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn make_packet_branches(input: &ItemTrait, module_name: &Ident) -> Vec<TokenStream> {
        input
        .methods()
        .map(|method| {
            let is_stream = is_stream(&method.sig.output);
            let variant_name = &method.sig.ident;

            let call_name = Ident::new(&(method.sig.ident.to_string() + "Call"), method.sig.ident.span());
            let reponse_name = Ident::new(&(method.sig.ident.to_string() + "Response"), method.sig.ident.span());
            let args_name: Vec<_> = method.sig.inputs.iter().filter_map(|arg| {
                if let FnArg::Typed(arg) = arg {
                    Some(&arg.pat)
                } else {
                    None
                }
            }).collect();

            if is_stream {
                quote! {
                    #module_name::MethodCall::#variant_name(#module_name::#call_name { #(#args_name,)* }) => {
                        let stream = match Self::#variant_name(&this, #(#args_name,)*).await {
                            ::core::result::Result::Ok(x) => x,
                            ::core::result::Result::Err(e) => {
                                send_packet!(#module_name::Response::Error(::std::format!("{e}")));
                                send_packet!(#module_name::Response::EndOfStream);
                                return;
                            }
                        };
                        let mut stream = ::std::pin::pin!(stream);
                        while let ::core::option::Option::Some(response) = ::ipc::futures::StreamExt::next(&mut stream).await {
                            send_packet!(match response {
                                ::core::result::Result::Ok(x) => #module_name::Response::#variant_name(#module_name::#reponse_name(x)),
                                ::core::result::Result::Err(e) => #module_name::Response::Error(::std::format!("{e}")),
                            });
                        }
                        send_packet!(#module_name::Response::EndOfStream);
                    }
                }
            } else {
                quote! {
                    #module_name::MethodCall::#variant_name(#module_name::#call_name { #(#args_name,)* }) => {
                        send_packet!(match Self::#variant_name(&this, #(#args_name,)*).await {
                            ::core::result::Result::Ok(x) => #module_name::Response::#variant_name(#module_name::#reponse_name(x)),
                            ::core::result::Result::Err(e) => #module_name::Response::Error(::std::format!("{e}")),
                        });
                    }
                }
            }
        })
        .collect()
    }
}
