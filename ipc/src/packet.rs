use std::sync::Arc;

use ipc_macros::{Read, Write};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt, BufWriter},
    sync::Mutex,
};

use crate::rw::Write;

#[derive(Debug, Clone)]
pub struct PacketSender<TX: AsyncWrite + Unpin + Send> {
    inner: Arc<Mutex<BufWriter<TX>>>,
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

impl<TX: AsyncWrite + Unpin + Send> PacketSender<TX> {
    pub fn new(stream: TX) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BufWriter::new(stream))),
        }
    }

    pub async fn write<T>(&self, payload: Clientbound<T>) -> anyhow::Result<()>
    where
        T: Write + Sync,
        T::Error: Send + Sync + 'static,
        anyhow::Error: From<T::Error>,
    {
        let mut inner = self.inner.lock().await;
        Write::write(&payload, &mut *inner).await?;
        inner.flush().await?;

        // drop "early" to satisfy clippy
        drop(inner);
        Ok(())
    }

    pub async fn write_with_id<T>(&self, id: u64, payload: T) -> anyhow::Result<()>
    where
        T: Write + Send,
        anyhow::Error: From<T::Error>,
    {
        let mut inner = self.inner.lock().await;
        id.write(&mut *inner).await?;
        payload.write(&mut *inner).await?;
        inner.flush().await?;

        // drop "early" to satisfy clippy
        drop(inner);
        Ok(())
    }
}
