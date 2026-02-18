use async_stream::try_stream;
use futures::Stream;
use log::{error, trace};
use std::{
    collections::HashMap,
    fmt::{self, Debug},
    io::{self, ErrorKind},
    os::unix::net::{SocketAddr, UnixStream as StdUnixStream},
    pin::Pin,
    result::Result as StdResult,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader},
    net::{
        UnixStream,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    spawn,
    sync::{RwLock, mpsc},
};

use crate::{Read, Result, Write};

use super::{Clientbound, Error, PacketSender, StreamPacket};

type Callback<RX> = Box<
    dyn for<'a> Fn(&'a mut BufReader<RX>) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>
        + Send
        + Sync,
>;

pub struct Client<RX: AsyncRead + Unpin + Send + 'static, TX: AsyncWrite + Unpin + Send> {
    next_call_id: Arc<AtomicU64>,
    packet_sender: PacketSender<TX>,
    callbacks: Arc<RwLock<HashMap<u64, Callback<RX>>>>,
}

impl Client<OwnedReadHalf, OwnedWriteHalf> {
    /// Create a connection from a unix socket address
    ///
    /// # Errors
    ///
    /// This function will return an error if the socket can't be created,
    /// connected to the address, set in non-blocking mode or converted to
    /// a tokio socket.
    pub fn from_unix_address(address: &SocketAddr) -> io::Result<Self> {
        let stream = StdUnixStream::connect_addr(address)?;
        stream.set_nonblocking(true)?;
        let (rx, tx) = UnixStream::from_std(stream)?.into_split();

        Ok(Self::new(rx, tx))
    }
}

impl<RX, TX> Client<RX, TX>
where
    RX: AsyncRead + Unpin + Send,
    TX: AsyncWrite + Unpin + Send + Sync,
{
    fn new(rx: RX, tx: TX) -> Self {
        let callbacks: Arc<RwLock<HashMap<u64, Callback<RX>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let callbacks_copy = Arc::clone(&callbacks);
        let mut rx = BufReader::new(rx);

        spawn(async move {
            let callbacks = callbacks_copy;
            loop {
                match u64::read(&mut rx).await {
                    Ok(x) => {
                        let mut callbacks = callbacks.write().await;
                        // None if the caller was cancelled
                        if let Some(callback) = callbacks.get(&x)
                            && callback(&mut rx).await
                        {
                            // callback has decided that it should be removed
                            callbacks.remove(&x);
                        }
                    }
                    Err(e) => {
                        if e.kind() != ErrorKind::UnexpectedEof {
                            error!("Error while receiving response: {e}");
                        }
                        // drop all callbacks (and it turn, channel receiving ends), allowing call to detect the crash
                        callbacks.write().await.clear();
                        break;
                    }
                }
                // putting behind explicit debug-only gate because it locks the RwLock
                if cfg!(debug_assertions) {
                    trace!("Callbacks remaining: {:?}", callbacks.read().await.keys());
                }
            }
        });

        Self {
            next_call_id: Arc::new(AtomicU64::new(0)),
            packet_sender: PacketSender::new(tx),
            callbacks,
        }
    }

    pub async fn call<T, R>(&self, method: T) -> Result<R>
    where
        T: Write + Send + Sync,
        T::Error: Send + Sync + 'static,
        R: Read + Send + Sync + 'static,
        R::Error: Send + Sync,
        anyhow::Error: From<T::Error> + From<R::Error>,
    {
        let (tx, mut rx) = mpsc::channel(1);
        self.call_base(method, move |rx| {
            let tx = tx.clone();
            Box::pin(async move {
                // error if receiving end is closed, i.e call is cancelled, ignore it
                let _ = tx
                    .send(R::read(rx).await.map_err(|e| Error::Read(e.into())))
                    .await;
                // remove this callback, we only expect one response
                true
            })
        })
        .await?;

        // recv returns None if tx is dropped, will happen if the receiving socket
        // is closed
        rx.recv().await.ok_or(Error::ConnectionBroken).flatten()
    }

    pub async fn long_call<T, R, E>(
        &self,
        method: T,
    ) -> Result<StdResult<impl Stream<Item = Result<R>> + use<T, R, E, RX, TX>, E>>
    where
        T: Write + Send + Sync,
        T::Error: Send + Sync + 'static,
        R: Read + Send + Sync + 'static,
        R::Error: Send + Sync,
        E: Read + Send + Sync + 'static + Debug,
        E::Error: Send + Sync + 'static,
        anyhow::Error: From<T::Error> + From<R::Error> + From<E::Error>,
    {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let _ = self
            .call_base(method, move |rx| {
                let tx = tx.clone();
                Box::pin(async move {
                    let value = StreamPacket::<R, E>::read(rx).await.map_err(Error::Read);

                    let is_end_packet = matches!(
                        value,
                        Ok(StreamPacket::EndOfStream | StreamPacket::Error(_)) | Err(_)
                    );
                    if tx.send(value).is_err() {
                        // receiving end is closed, i.e call is cancelled, remove this callback
                        return true;
                    }

                    // remove the callback if we don't expect new packets
                    is_end_packet
                })
            })
            .await?;

        let first_packet = rx.recv().await.ok_or(Error::ConnectionBroken)??;
        let first_value = match first_packet {
            StreamPacket::Value(x) => Some(x),
            StreamPacket::EndOfStream => None,
            StreamPacket::Error(e) => return Ok(Err(e)),
        };

        Ok(Ok(try_stream! {
            if let Some(first_value) = first_value {
                yield first_value;
            } else {
                // first packet was EndOfStream
                return;
            }

            loop {
                let value = match rx.recv().await.ok_or(Error::ConnectionBroken)?? {
                    StreamPacket::Value(x) => x,
                    StreamPacket::Error(e) => {
                        error!("Unexpected early error after receiving value: {e:?}");
                        break;
                    },
                    StreamPacket::EndOfStream => break,
                };
                yield value;
            }
        }))
    }

    async fn call_base<T, F>(&self, method: T, callback: F) -> Result<u64>
    where
        T: Write + Send + Sync,
        T::Error: Send + Sync + 'static,
        anyhow::Error: From<T::Error>,
        F: for<'a> Fn(&'a mut BufReader<RX>) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        let call_id = self.next_call_id.fetch_add(1, Ordering::Relaxed);

        // add callback before writing, otherwise we could read the response
        // before having the callback in place
        self.callbacks
            .write()
            .await
            .insert(call_id, Box::new(callback));
        if let Err(e) = self
            .packet_sender
            .write::<Clientbound<T>>(Clientbound {
                call_id,
                payload: method,
            })
            .await
        {
            self.callbacks.write().await.remove(&call_id);
            return Err(super::Error::Read(e));
        }

        Ok(call_id)
    }
}

impl<RX, TX> Clone for Client<RX, TX>
where
    RX: AsyncRead + Unpin + Send + 'static,
    TX: AsyncWrite + Unpin + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            next_call_id: Arc::clone(&self.next_call_id),
            packet_sender: self.packet_sender.clone(),
            callbacks: Arc::clone(&self.callbacks),
        }
    }
}

impl<RX, TX> Debug for Client<RX, TX>
where
    RX: AsyncRead + Unpin + Send + 'static,
    TX: AsyncWrite + Unpin + Send + Sync + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("next_call_id", &self.next_call_id)
            .field("packet_sender", &self.packet_sender)
            .field("callbacks", &"<callbacks>")
            .finish()
    }
}
