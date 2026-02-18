use std::sync::Arc;

use async_stream::stream;
use futures::Stream;
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufWriter},
    sync::RwLock,
};

use crate::{Read, Write};

pub mod client;
pub mod server;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while reading packet: {0}")]
    Read(#[source] anyhow::Error),
    #[error("The connection to the server was broken or the server closed")]
    ConnectionBroken,
}

/// Packet going from the client to the server
#[derive(Debug, Clone, Read, Write)]
pub struct Clientbound<T> {
    /// A unique id identifying this method call
    pub call_id: u64,
    pub payload: T,
}

/// Packet going from the server to the client
#[derive(Debug, Clone, Read, Write)]
pub struct Serverbound<T> {
    /// Id of the method call that this packet is a response to
    pub call_id: u64,
    pub payload: T,
}

/// Packet wrapper for streamed responses
#[derive(Debug, Clone, Read, Write)]
pub enum StreamPacket<T, E> {
    /// A value from the stream
    Value(T),
    /// The early error, if any (should only be present in the first packet)
    Error(E),
    /// The stream has ended normally
    EndOfStream,
}

#[derive(Debug)]
pub struct PacketSender<TX: AsyncWrite> {
    inner: Arc<RwLock<BufWriter<TX>>>,
}

impl<TX: AsyncWrite + Unpin + Send + Sync> PacketSender<TX> {
    pub fn new(stream: TX) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BufWriter::new(stream))),
        }
    }

    pub async fn write<T>(&self, payload: T) -> anyhow::Result<()>
    where
        T: Write,
        anyhow::Error: From<T::Error>,
    {
        let mut inner = self.inner.write().await;
        Write::write(&payload, &mut *inner).await?;
        inner.flush().await?;

        // drop "early" to satisfy clippy
        drop(inner);
        Ok(())
    }
}

impl<TX: AsyncWrite + Unpin + Send> Clone for PacketSender<TX> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[derive(Debug)]
pub struct PacketReceiver<RX: AsyncRead> {
    inner: RX,
}

impl<RX: AsyncRead + Unpin + Send> PacketReceiver<RX> {
    pub const fn new(stream: RX) -> Self {
        Self { inner: stream }
    }

    pub fn receive_stream<T>(mut self) -> impl Stream<Item = Result<T, T::Error>>
    where
        T: Read,
    {
        stream! {
            loop {
                yield T::read(&mut self.inner).await;
            }
        }
    }
}
