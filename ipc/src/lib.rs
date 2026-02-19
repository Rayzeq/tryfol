#![feature(never_type)]

// allow usage of macros from ipc-macros in this crate
extern crate self as ipc;

use core::result::Result as StdResult;

// re-exports for the macro
#[doc(hidden)]
pub use anyhow;
#[doc(hidden)]
pub use futures;
#[doc(hidden)]
pub use log;
#[doc(hidden)]
pub use tokio;

mod protocol;
mod rw;

/// Derive macro for implementing [`Read`].
#[doc(inline)]
pub use ipc_macros::Read;
/// Derive macro for implementing [`Write`].
#[doc(inline)]
pub use ipc_macros::Write;
pub use ipc_macros::protocol;
pub use protocol::Error;
pub use rw::{InvalidDiscriminantError, Read, Write};

pub type Result<T> = StdResult<T, Error>;

#[doc(hidden)]
pub mod __private {
    pub use super::protocol::{
        Clientbound, PacketReceiver, Serverbound, StreamPacket, Writable,
        client::Client,
        server::{run_server, stream_with_id},
    };
}
