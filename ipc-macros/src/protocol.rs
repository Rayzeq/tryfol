use darling::FromMeta;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ItemTrait, TraitItem, TraitItemFn, Type, parse_quote};

use crate::utils::{FunctionEditor, IdentEditor, ParseType, TraitEditor};

#[derive(Default, FromMeta)]
#[darling(default)]
pub struct Arguments {
    #[darling(default)]
    abstract_socket: Option<String>,
    #[darling(default)]
    client_name: Option<String>,
    #[darling(default)]
    server_name: Option<String>,
}

pub struct Protocol {
    input: ItemTrait,

    abstract_socket: Option<String>,

    client_name: Ident,
    server_name: Ident,
    module_name: Ident,
}

impl Protocol {
    pub fn new(input: ItemTrait, args: Arguments) -> Self {
        let base_name = &input.ident;

        let client_name = base_name.with_name(
            &args
                .client_name
                .unwrap_or_else(|| base_name.to_string() + "Client"),
        );
        let server_name = base_name.with_name(
            &args
                .server_name
                .unwrap_or_else(|| base_name.to_string() + "Server"),
        );
        let module_name =
            base_name.with_name(&("__".to_owned() + &base_name.to_string() + "_inner"));

        Self {
            input,

            abstract_socket: args.abstract_socket,

            client_name,
            server_name,
            module_name,
        }
    }

    pub fn make(&self) -> TokenStream {
        let (
            client_methods,
            call_structs,
            call_variants,
            response_structs,
            response_variants,
            packet_branches,
        ) = self
            .input
            .methods()
            .map(|method| {
                let name = &method.sig.ident;
                let return_type = get_response_type(method);
                let call_name = name.with_name(&(name.to_string() + "Call"));
                let response_name = name.with_name(&(name.to_string() + "Response"));

                (
                    self.make_client_method(method, name, &call_name, &return_type),
                    Self::make_call_struct(method, name, &call_name, &response_name),
                    quote! {
                        #name(#call_name)
                    },
                    Self::make_response_struct(method, name, &response_name, &return_type),
                    quote! {
                        #name(#response_name)
                    },
                    self.make_packet_branch(method, name, &call_name, &response_name),
                )
            })
            .multiunzip::<(Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>, Vec<_>)>();

        let module_name = &self.module_name;
        let client_trait = self.make_client_trait();
        let client = self.make_client(&client_methods);
        let server_trait = self.make_server_trait(&packet_branches);
        quote! {
            #[allow(non_snake_case, non_camel_case_types)]
            mod #module_name {
                use super::*;

                #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
                pub enum MethodCall {
                    #(#call_variants,)*
                }

                impl ::ipc::AnyCall for MethodCall {
                    type Response = Response;
                }

                #(#call_structs)*

                #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
                pub enum Response {
                    #(#response_variants,)*
                    EndOfStream,
                    Error(String),
                }

                impl ::ipc::AnyResponse for Response {}

                #(#response_structs)*
            }

            #client_trait
            #client
            #server_trait
        }
    }

    fn make_call_struct(
        method: &TraitItemFn,
        name: &Ident,
        call_name: &Ident,
        response_name: &Ident,
    ) -> TokenStream {
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
                impl ::ipc::LongMethod for #call_name {
                    type Response = #response_name;
                }
            }
        } else {
            quote! {
                impl ::ipc::Method for #call_name {
                    type Response = #response_name;
                }
            }
        };

        quote! {
            #[derive(::core::fmt::Debug, ::core::clone::Clone, ::ipc::Read, ::ipc::Write)]
            pub struct #call_name {
                #(#fields,)*
            }

            impl ::core::convert::From<#call_name> for MethodCall {
                fn from(value: #call_name) -> Self {
                    Self::#name(value)
                }
            }

            #conditional
        }
    }

    fn make_response_struct(
        method: &TraitItemFn,
        name: &Ident,
        response_name: &Ident,
        field_type: &Type,
    ) -> TokenStream {
        let conditional = if is_stream(&method.sig.output) {
            quote! {
                impl ::core::convert::TryFrom<Response> for ::core::option::Option<#response_name> {
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
                impl ::core::convert::TryFrom<Response> for #response_name {
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
            pub struct #response_name(pub #field_type);

            impl ::ipc::Response for #response_name {
                type Inner = #field_type;

                fn into_inner(self) -> Self::Inner {
                    self.0
                }
            }

            #conditional
        }
    }

    fn make_client_method(
        &self,
        method: &TraitItemFn,
        name: &Ident,
        call_name: &Ident,
        return_type: &Type,
    ) -> TraitItemFn {
        let module_name = &self.module_name;
        let args = &method.sig.inputs;

        let construct: Vec<_> = method
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let FnArg::Typed(arg) = arg {
                    Some(arg.pat.clone())
                } else {
                    None
                }
            })
            .collect();

        if is_stream(&method.sig.output) {
            parse_quote! {
                async fn #name(#args) -> ::ipc::Result<impl ::ipc::futures::Stream<Item = ::ipc::Result<#return_type, #module_name::Response>>, #module_name::Response>
                {
                    ::ipc::Connection::long_call(&self.inner, #module_name::#call_name { #(#construct,)* }).await
                }
            }
        } else {
            parse_quote! {
                async fn #name(#args) -> ::ipc::Result<#return_type, #module_name::Response> {
                    ::ipc::Connection::call(&self.inner, #module_name::#call_name { #(#construct,)* }).await
                }
            }
        }
    }

    fn make_client_trait(&self) -> ItemTrait {
        let mut input = self.input.clone();
        let module_name = &self.module_name;
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

    fn make_client(&self, methods: &[TraitItemFn]) -> TokenStream {
        let vis = &self.input.vis;
        let module_name = &self.module_name;
        let base_name = &self.input.ident;
        let client_name = &self.client_name;

        let mut additional_impls = Vec::new();
        if let Some(socket_name) = &self.abstract_socket {
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

            impl<T: ::ipc::tokio::io::AsyncWriteExt + ::core::marker::Unpin + ::core::marker::Send> #base_name for #client_name<T> {
                #(#methods)*
            }

            #(#additional_impls)*
        }
    }

    fn make_server_trait(&self, packet_branches: &[TokenStream]) -> ItemTrait {
        let mut input = self.input.clone();
        input.set_name(self.server_name.clone());
        input
            .attrs
            .push(parse_quote!(#[allow(clippy::module_name_repetitions)]));

        let handle_client_method = self.make_handle_client_method(packet_branches);
        input.items.push(handle_client_method);

        if let Some(socket_name) = &self.abstract_socket {
            let server_trait_name = &self.server_name;
            let serve_method = parse_quote! {
                fn serve(self) -> impl ::std::future::Future<Output = ::std::io::Result<::std::convert::Infallible>>
                        + ::core::marker::Send
                where
                    Self: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static
                {
                    struct CancelGuard {
                        tasks: ::std::vec::Vec<::ipc::tokio::task::JoinHandle<()>>,
                    }

                    impl CancelGuard {
                        pub fn add(&mut self, task: ::ipc::tokio::task::JoinHandle<()>) {
                            self.tasks.push(task);
                        }

                        pub fn cleanup(&mut self) {
                            self.tasks.retain(|task| !task.is_finished());
                        }
                    }

                    impl ::core::ops::Drop for CancelGuard {
                        fn drop(&mut self) {
                            for task in &self.tasks {
                                task.abort();
                            }
                        }
                    }

                    async move {
                        const TIMEOUT: ::core::time::Duration = ::core::time::Duration::from_secs(60);

                        let addr = <::std::os::unix::net::SocketAddr as ::std::os::linux::net::SocketAddrExt>::from_abstract_name(#socket_name)?;
                        let listener = ::std::os::unix::net::UnixListener::bind_addr(&addr)?;
                        listener.set_nonblocking(true)?;
                        let listener = ::ipc::tokio::net::UnixListener::from_std(listener)?;

                        let mut guard = CancelGuard { tasks: ::std::vec::Vec::new() };

                        let timer = ::ipc::tokio::time::sleep(TIMEOUT);
                        ::ipc::tokio::pin!(timer);
                        loop {
                            ::ipc::tokio::select! {
                                result = listener.accept() => {
                                    match result {
                                        ::core::result::Result::Ok((stream, _)) => {
                                            let (rx, tx) = stream.into_split();
                                            let this = <Self as ::core::clone::Clone>::clone(&self);
                                            let task = ::ipc::tokio::task::spawn(<Self as #server_trait_name>::handle_client(this, rx, tx));
                                            guard.add(task);
                                        }
                                        ::core::result::Result::Err(e) => {
                                            ::log::error!("Error accepting client: {e}");
                                        }
                                    }
                                },
                                () = &mut timer => {
                                    // Timeout reached, cleanup dead tasks
                                    guard.cleanup();
                                    timer.as_mut().reset(::ipc::tokio::time::Instant::now() + TIMEOUT);
                                }
                            }
                        }
                    }
                }
            };
            input.items.push(serve_method);
        }

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

    fn make_handle_client_method(&self, branches: &[TokenStream]) -> TraitItem {
        let module_name = &self.module_name;

        parse_quote! {
            fn handle_client(
                self,
                mut rx: impl ::ipc::tokio::io::AsyncRead + ::core::marker::Unpin + ::core::marker::Send,
                tx: impl ::ipc::tokio::io::AsyncWrite + ::core::marker::Unpin + ::core::marker::Send + 'static,
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

    fn make_packet_branch(
        &self,
        method: &TraitItemFn,
        variant_name: &Ident,
        call_name: &Ident,
        response_name: &Ident,
    ) -> TokenStream {
        let module_name = &self.module_name;
        let is_stream = is_stream(&method.sig.output);

        let args_name: Vec<_> = method
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let FnArg::Typed(arg) = arg {
                    Some(&arg.pat)
                } else {
                    None
                }
            })
            .collect();

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
                            ::core::result::Result::Ok(x) => #module_name::Response::#variant_name(#module_name::#response_name(x)),
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
                        ::core::result::Result::Ok(x) => #module_name::Response::#variant_name(#module_name::#response_name(x)),
                        ::core::result::Result::Err(e) => #module_name::Response::Error(::std::format!("{e}")),
                    });
                }
            }
        }
    }
}

fn is_stream(ty: &impl ParseType) -> bool {
    ty.as_result()
        .is_some_and(|result| result.ok().as_stream().is_some())
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
