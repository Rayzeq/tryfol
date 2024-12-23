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

pub use client::{Connection, ResponseStream};
pub use errors::ClientError;
use rw::{Read, Write};

pub type Result<T, E> = core::result::Result<T, ClientError<E>>;

/// Type of a method call, it contains the arguments of the method
pub trait Method: Send + Sync {
    type Response;
}

/// Type of a method call that returns a stream, it contains the arguments of the method
pub trait LongMethod: Send + Sync {
    type Response;
}

/// Trait implemented by the enum containing all possible calls
pub trait AnyCall: Send + Sync + Write<Error: Send + Sync + 'static> {
    type Response: AnyResponse;
}

/// Trait implemented by the enum containing all possible reponses
pub trait AnyResponse: Debug + Send + Sync + Read<Error: Send + Sync> + 'static {}
