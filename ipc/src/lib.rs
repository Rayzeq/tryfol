// allow usage of the macros of ipc-macros in this crate
extern crate self as ipc;

pub use anyhow;
pub use futures;
pub use ipc_macros::{protocol, Read, Write};
pub use log;
use std::fmt::Debug;
pub use tokio;

mod client;
mod errors;
pub mod packet;
pub mod rw;

pub use client::Connection;
pub use errors::ClientError;
use rw::{Read, Write};

pub type Result<T, E> = core::result::Result<T, ClientError<E>>;

/// Type of a method call, it contains the arguments of the method
pub trait Method: Send + Sync {
    type Response: Response;
}

/// Type of a method call that returns a stream, it contains the arguments of the method
pub trait LongMethod: Send + Sync {
    type Response: Response;
}

/// Trait implemented by the enum containing all possible calls
pub trait AnyCall: Send + Sync + Write<Error: Send + Sync + 'static> {
    type Response: AnyResponse;
}

/// Type of the response of one precise method call
pub trait Response {
    type Inner;
    fn into_inner(self) -> Self::Inner;
}

/// Trait implemented by the enum containing all possible reponses
pub trait AnyResponse: Debug + Send + Sync + Read<Error: Send + Sync> + 'static {}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused)]
    #[protocol]
    pub trait ProtocolWithTwoIdenticalReturnType {
        async fn func1(&self) -> ipc::Result<u32>;
        async fn func2(&self) -> ipc::Result<u32>;
    }
}
