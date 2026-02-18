use proc_macro::{Diagnostic, Level};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Ident, ReturnType, parse_quote, punctuated::Punctuated, spanned::Spanned};

use super::{Protocol, ProtocolMethod};

impl Protocol {
    pub fn generate(mut self) -> TokenStream {
        self.sanitize();
        let name = &self.name;
        let module_name = &self.module_name;
        let server_name = &self.server_name;
        let client_name = &self.client_name;
        let visibility = &self.visibility;

        let call_structs = self.generate_call_structs();
        let server_trait = self.generate_server_trait();
        let client_trait = self.generate_client_trait();
        let client = self.generate_client();

        quote! {
            #[allow(non_snake_case, non_camel_case_types)]
            #[doc(hidden)]
            mod #module_name {
                use super::*;

                #call_structs
                #server_trait
                #client_trait
                #client
            }

            #visibility use #module_name::#server_name;
            #visibility use #module_name::#client_name;
            #visibility use #module_name::#name;
        }
    }

    /// Emit errors for forbidden things and remove them from the code to avoid creating even more errors
    fn sanitize(&mut self) {
        if !self.generics.params.is_empty() {
            Diagnostic::spanned(self.generics.span().unwrap(), Level::Error, "protocols cannot have generics yet").emit();
        }

        for method in &mut self.methods {
            let generics = &mut method.inner_mut().sig.generics;
            if !generics.params.is_empty() {
                Diagnostic::spanned(generics.span().unwrap(), Level::Error, "protocol methods cannot contain generics or lifetimes").emit();
            }
            // delete generics to prevent a lot of irrelevant errors from showing up
            generics.params = Punctuated::new();
            generics.where_clause = None;
        }
    }

    fn generate_call_structs(&self) -> TokenStream {
        let (variants, structs): (Vec<_>, Vec<_>) = self
            .methods
            .iter()
            .map(|method| {
                let name = &method.inner().sig.ident;
                let struct_name = Ident::new(&format!("{name}Call"), Span::mixed_site());
                let fields = method.inner().sig.inputs.iter().filter_map(|arg| {
                    let FnArg::Typed(arg) = arg else {
                        return None;
                    };
                    let name = &arg.pat;
                    let ty = &arg.ty;
                    Some(quote!(pub #name: #ty))
                });

                (
                    quote!(#name(#struct_name)),
                    quote! {
                        #[derive(::ipc::Read, ::ipc::Write)]
                        struct #struct_name {
                            #(#fields),*
                        }
                    },
                )
            })
            .collect();

        quote! {
            #[derive(::ipc::Read, ::ipc::Write)]
            enum MethodCall {
                #(#variants),*
            }

            #(#structs)*
        }
    }

    fn generate_server_trait(&self) -> TokenStream {
        let methods: Vec<_> = self.methods.iter().cloned().map(|method| {
            let mut method = match method {
                ProtocolMethod::SimpleCall(mut method) => {
                    let output = match &method.sig.output {
                        ReturnType::Default => parse_quote!(()),
                        ReturnType::Type(_, output) => (*output).clone(),
                    };

                    method.sig.output = parse_quote!(-> impl ::core::future::Future<Output = #output> + ::core::marker::Send);
                    method
                }
                ProtocolMethod::LongCall{ mut method, early_error } => {
                    let output = match &method.sig.output {
                        ReturnType::Default => parse_quote!(()),
                        ReturnType::Type(_, output) => (*output).clone(),
                    };

                    if let Some(error) = early_error {
                        method.sig.output = parse_quote!(-> impl ::core::future::Future<Output = ::core::result::Result<impl ::ipc::futures::Stream<Item = #output> + ::core::marker::Send, #error>> + ::core::marker::Send);
                    } else {
                        method.sig.output = parse_quote!(-> impl ::core::future::Future<Output = impl ::ipc::futures::Stream<Item = #output> + ::core::marker::Send> + ::core::marker::Send);
                    }
                    method
                },
            };
            if let Some(FnArg::Receiver(receiver)) = method.sig.inputs.first_mut() {
                receiver.reference = None;
                receiver.ty = parse_quote!(Self);
            }
            method.sig.asyncness = None;
            method
        }).collect();

        let server_name = &self.server_name;
        let attributes = &self.attributes;
        let generics = &self.generics.params;
        let where_clause = &self.generics.where_clause;
        let supertraits = &self.supertraits;

        let (serve_method, handle_client_method) = self
            .abstract_socket
            .as_ref()
            .map(|socket_name| self.generate_serve_method(socket_name))
            .map_or((None, None), |(serve, handle_client)| (Some(serve), Some(handle_client)));

        quote! {
            #(#attributes)*
            pub trait #server_name<#generics>: #supertraits #where_clause {
                #(#methods)*

                #serve_method
            }

            #handle_client_method
        }
    }

    fn generate_serve_method(&self, socket_name: &str) -> (TokenStream, TokenStream) {
        let server_name = &self.server_name;
        let (variable_creation, read_branch, select_branch): (Vec<_>, Vec<_>, Vec<_>) = self.methods.iter().map(|method| {
            let name = &method.inner().sig.ident;
            let calls_name = Ident::new(&format!("{name}_calls"), Span::mixed_site());
            let call_struct_name = Ident::new(&format!("{name}Call"), Span::mixed_site());

            let args_name: Vec<_> = method
                .inner()
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

            let variable_creation = quote! {
                let mut #calls_name = ::ipc::futures::prelude::stream::FuturesUnordered::new();
            };
            let read_branch = quote! {
                ::core::result::Result::Ok(::ipc::__private::Clientbound { call_id, payload: MethodCall::#name(#call_struct_name { #(#args_name),* }) }) => ::ipc::futures::prelude::stream::FuturesUnordered::push(&mut #calls_name, {
                    let server = server.clone();
                    async move { (call_id, #server_name::#name(server, #(#args_name),*).await) }
                })
            };
            match method {
                ProtocolMethod::SimpleCall(_) => {
                    let select_branch = quote! {
                        ::core::option::Option::Some((id, result)) = ::ipc::futures::StreamExt::next(&mut #calls_name), if !::ipc::futures::prelude::stream::FuturesUnordered::is_empty(&#calls_name) => {
                            send_packet!(tx, id, result);
                        }
                    };

                    (variable_creation, read_branch, select_branch)
                },
                ProtocolMethod::LongCall { early_error, .. } => {
                    let streams_name = Ident::new(&format!("{name}_streams"), Span::mixed_site());
                    let select_branch = if early_error.is_some() {
                        quote! {
                            ::core::option::Option::Some((id, stream)) = ::ipc::futures::StreamExt::next(&mut #calls_name), if !::ipc::futures::prelude::stream::FuturesUnordered::is_empty(&#calls_name) => {
                                match stream {
                                    ::core::result::Result::Ok(x) => ::ipc::futures::prelude::stream::SelectAll::push(&mut #streams_name, ::std::boxed::Box::pin(::ipc::__private::stream_with_id(id, x))),
                                    ::core::result::Result::Err(e) => { send_packet!(tx, id, ::ipc::__private::StreamPacket::<!, _>::Error(e)); }
                                }
                            }
                        }
                    } else {
                        quote! {
                            ::core::option::Option::Some((id, stream)) = ::ipc::futures::StreamExt::next(&mut #calls_name), if !::ipc::futures::prelude::stream::FuturesUnordered::is_empty(&#calls_name) => {
                                ::ipc::futures::prelude::stream::SelectAll::push(&mut #streams_name, ::std::boxed::Box::pin(::ipc::__private::stream_with_id(id, stream)));
                            }
                        }
                    };

                    let variable_creation = quote! {
                        #variable_creation
                        let mut #streams_name = ::ipc::futures::prelude::stream::SelectAll::new();
                    };
                    let select_branches = quote! {
                        #select_branch,
                        ::core::option::Option::Some((id, result)) = ::ipc::futures::StreamExt::next(&mut #streams_name), if !::ipc::futures::prelude::stream::SelectAll::is_empty(&#streams_name) => {
                            send_packet!(tx, id, result);
                        }
                    };

                    (variable_creation, read_branch, select_branches)
                }
            }
        }).collect();

        let handle_client_method = quote! {
            async fn handle_client(
                server: impl #server_name + ::core::clone::Clone,
                rx: ::ipc::__private::PacketReceiver<::ipc::tokio::net::unix::OwnedReadHalf>,
                mut tx: ::ipc::tokio::io::BufWriter<::ipc::tokio::net::unix::OwnedWriteHalf>,
            ) {
                macro_rules! send_packet {
                    ($tx:expr, $id:expr, $payload:expr) => {
                        let packet = ::ipc::__private::Serverbound {
                            call_id: $id,
                            payload: $payload,
                        };

                        let result = ::ipc::Write::write(&packet, &mut tx).await;
                        if let ::core::result::Result::Err(e) = result {
                            if let ::core::option::Option::Some(e) =
                                ::ipc::anyhow::Error::downcast_ref::<::std::io::Error>(&e)
                                && ::std::io::Error::kind(&e) == ::std::io::ErrorKind::BrokenPipe
                            {
                                // client disconnected (may have crashed or cancelled all calls)
                                return;
                            }
                            ::ipc::log::error!("Error while sending packet to client: {e}");
                        } else {
                            // we don't really care if we can't flush
                            let _ = ::ipc::tokio::io::AsyncWriteExt::flush(&mut $tx).await;
                        }
                    };
                }

                #(#variable_creation)*

                let read_stream = ::ipc::__private::PacketReceiver::receive_stream::<::ipc::__private::Clientbound<MethodCall>>(rx);
                ::ipc::tokio::pin!(read_stream);

                loop {
                    ::ipc::tokio::select! {
                        ::core::option::Option::Some(packet) = ::ipc::futures::StreamExt::next(&mut read_stream) => match packet {
                            #(#read_branch,)*
                            ::core::result::Result::Err(e) => {
                                if let ::core::option::Option::Some(e) = ::ipc::anyhow::Error::downcast_ref::<::std::io::Error>(&e)
                                    && ::std::io::Error::kind(e) == ::std::io::ErrorKind::UnexpectedEof
                                {
                                    // client quitted normally
                                    break;
                                }
                                ::ipc::log::error!("Error receiving message from client: {e}");
                                break;
                            }
                        },
                        #(#select_branch,)*
                    }
                }
            }
        };

        let serve_method = quote! {
            fn serve(self) -> impl ::core::future::Future<Output = ::std::io::Result<!>> + ::core::marker::Send
            where
                Self: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static,
            {
                self.serve_with_abstract_socket(#socket_name)
            }

            fn serve_with_abstract_socket(self, socket: &str) -> impl ::core::future::Future<Output = ::std::io::Result<!>> + ::core::marker::Send
            where
                Self: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static,
            {
                async move {
                    let addr = <::std::os::unix::net::SocketAddr as ::std::os::linux::net::SocketAddrExt>::from_abstract_name(socket)?;
                    let listener = ::std::os::unix::net::UnixListener::bind_addr(&addr)?;
                    ::std::os::unix::net::UnixListener::set_nonblocking(&listener, true)?;
                    let listener = ::ipc::tokio::net::UnixListener::from_std(listener)?;

                    ::ipc::__private::run_server(self, listener, handle_client).await
                }
            }
        };

        (serve_method, handle_client_method)
    }

    fn generate_client_trait(&self) -> TokenStream {
        let methods: Vec<_> = self.methods.iter().cloned().map(|method| {
            let mut method = match method {
                ProtocolMethod::SimpleCall(mut method) => {
                    let output = match &method.sig.output {
                        ReturnType::Default => parse_quote!(()),
                        ReturnType::Type(_, output) => (*output).clone(),
                    };

                    method.sig.output = parse_quote!(-> impl ::core::future::Future<Output = ::ipc::Result<#output>> + ::core::marker::Send);
                    method
                }
                ProtocolMethod::LongCall{mut method, early_error} => {
                    let output = match &method.sig.output {
                        ReturnType::Default => parse_quote!(()),
                        ReturnType::Type(_, output) => (*output).clone(),
                    };

                    if let Some(error) = early_error {
                        method.sig.output = parse_quote!(-> impl ::core::future::Future<Output = ::ipc::Result<::core::result::Result<impl ::ipc::futures::Stream<Item = ::ipc::Result<#output>> + ::core::marker::Send, #error>>> + ::core::marker::Send);
                    } else {
                        method.sig.output = parse_quote!(-> impl ::core::future::Future<Output = ::ipc::Result<impl ::ipc::futures::Stream<Item = ::ipc::Result<#output>> + ::core::marker::Send>> + ::core::marker::Send);
                    }
                    method
                },
            };
            method.sig.asyncness = None;
            method
        }).collect();

        let name = &self.name;
        let attributes = &self.attributes;
        let generics = &self.generics;
        let supertraits = &self.supertraits;

        quote! {
            #(#attributes)*
            pub trait #name #generics: #supertraits {
                #(#methods)*
            }
        }
    }

    fn generate_client(&self) -> TokenStream {
        let methods: Vec<_> = self
            .methods
            .iter()
            .cloned()
            .map(|mut method| {
                let output = match &method.inner().sig.output {
                    ReturnType::Default => parse_quote!(()),
                    ReturnType::Type(_, output) => (*output).clone(),
                };

                match &mut method {
                    ProtocolMethod::SimpleCall(method) => {
                        method.sig.output = parse_quote!(-> ::ipc::Result<#output>);
                    }
                    ProtocolMethod::LongCall { method, early_error } => {
                        if let Some(error) = early_error {
                            method.sig.output =
                                parse_quote!(-> ::ipc::Result<::core::result::Result<impl ::ipc::futures::Stream<Item = ::ipc::Result<#output>> + ::core::marker::Send, #error>>);
                        } else {
                            method.sig.output = parse_quote!(-> ::ipc::Result<impl ::ipc::futures::Stream<Item = ::ipc::Result<#output>> + ::core::marker::Send>);
                        }
                    }
                }

                let signature = &method.inner().sig;
                let name = &signature.ident;
                let struct_name = Ident::new(&format!("{name}Call"), Span::mixed_site());
                let args = signature.inputs.iter().filter_map(|x| match x {
                    FnArg::Receiver(_) => None,
                    FnArg::Typed(x) => Some(&x.pat),
                });

                match &method {
                    ProtocolMethod::SimpleCall(_) => {
                        quote! {
                            #signature {
                                ::ipc::__private::Client::call(
                                    &self.inner,
                                    MethodCall::#name(#struct_name { #(#args),* })
                                ).await
                            }
                        }
                    }
                    ProtocolMethod::LongCall { early_error, .. } => {
                        if early_error.is_some() {
                            quote! {
                                #signature {
                                    ::ipc::__private::Client::long_call(
                                        &self.inner,
                                        MethodCall::#name(#struct_name { #(#args),* })
                                    ).await
                                }
                            }
                        } else {
                            quote! {
                                #signature {
                                    let ::core::result::Result::Ok(result) =::ipc::__private::Client::long_call::<_, _, !>(
                                        &self.inner,
                                        MethodCall::#name(#struct_name { #(#args),* })
                                    ).await?;
                                    Ok(result)
                                }
                            }
                        }
                    }
                }
            })
            .collect();

        let name = &self.client_name;
        let trait_name = &self.name;
        let attributes = &self.attributes;
        let impl_generics = self.generics.params.iter();
        let (_, ty_generics, where_clause) = self.generics.split_for_impl();

        let socket_impl = self.abstract_socket.as_ref().map(|socket| {
            quote! {
                impl #name<::ipc::tokio::net::unix::OwnedReadHalf, ::ipc::tokio::net::unix::OwnedWriteHalf> {
                    pub fn new() -> ::std::io::Result<Self> {
                        Self::new_with_abstract_socket(#socket)
                    }

                    pub fn new_with_abstract_socket(socket: &str) -> ::std::io::Result<Self> {
                        ::core::result::Result::Ok(Self {
                            inner: ::ipc::__private::Client::from_unix_address(&<::std::os::unix::net::SocketAddr as ::std::os::linux::net::SocketAddrExt>::from_abstract_name(socket)?)?,
                        })
                    }
                }
            }
        });

        // TODO: simplify, use less dyn, RwLock, ...
        //
        // Idea: pass function pointers pointing to reading functions instead of Box<dyn>
        quote! {
            #(#attributes)*
            pub struct #name<
                RX: ::ipc::tokio::io::AsyncRead + ::core::marker::Unpin + ::core::marker::Send + 'static,
                TX: ::ipc::tokio::io::AsyncWrite + ::core::marker::Unpin + ::core::marker::Send,

            > {
                inner: ::ipc::__private::Client<RX, TX>,
            }

            impl<
                RX: ::ipc::tokio::io::AsyncRead + ::core::marker::Unpin + ::core::marker::Send,
                TX: ::ipc::tokio::io::AsyncWrite + ::core::marker::Unpin + ::core::marker::Send + ::core::marker::Sync,
                #(#impl_generics,)*
            > #trait_name #ty_generics for #name<RX, TX> #where_clause {
                #(#methods)*
            }

            #socket_impl
        }
    }
}
