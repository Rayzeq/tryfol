use crate::{
    errors::ClientError,
    packet::{self, Clientbound, PacketSender},
    rw::{Read, Write},
    AnyCall, LongMethod, Method, Response,
};
use futures::{stream, Stream};
use log::{error, warn};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, ErrorKind},
    os::unix::net::{SocketAddr, UnixStream as StdUnixStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{unix::OwnedWriteHalf, UnixStream},
    spawn,
    sync::{mpsc, Mutex},
};

#[derive(Debug, Clone)]
pub struct Connection<T: AnyCall, TX: AsyncWrite + Unpin + Send> {
    stream: PacketSender<TX>,
    next_call_id: Arc<AtomicU64>,
    channels: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<T::Response>>>>,
}

impl<T: AnyCall> Connection<T, OwnedWriteHalf>
where
    anyhow::Error: From<<T as Write>::Error> + From<<T::Response as Read>::Error>,
{
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

impl<T: AnyCall, TX: AsyncWrite + Unpin + Send> Connection<T, TX>
where
    anyhow::Error: From<<T as Write>::Error> + From<<T::Response as Read>::Error>,
{
    pub fn new<RX: AsyncRead + Unpin + Send + 'static>(mut rx: RX, tx: TX) -> Self {
        let channels: Arc<Mutex<HashMap<_, mpsc::UnboundedSender<_>>>> = Arc::default();
        let channels2 = Arc::clone(&channels);

        spawn(async move {
            loop {
                match packet::Serverbound::<T::Response>::read(&mut rx).await {
                    Ok(x) => {
                        let mut channels = channels2.lock().await;
                        if let Some(sender) = channels.get(&x.call_id) {
                            if sender.send(x.payload).is_err() {
                                // the receiver might have been dropped if the future has been cancelled
                                channels.remove(&x.call_id);
                                // early drop to make clippy happy
                                drop(channels);
                            }
                        } else {
                            warn!("Packet response has no receiver: {x:?}");
                        }
                    }
                    Err(e) => {
                        if e.downcast_ref::<io::Error>()
                            .is_none_or(|e| e.kind() != ErrorKind::UnexpectedEof)
                        {
                            error!("Error while receiving response: {e}");
                        }
                        channels2.lock().await.clear();
                        break;
                    }
                };
            }
        });

        Self {
            stream: PacketSender::new(tx),
            next_call_id: Arc::new(AtomicU64::new(0)),
            channels,
        }
    }

    pub async fn call<M>(
        &self,
        method: M,
    ) -> Result<<M::Response as Response>::Inner, ClientError<T::Response>>
    where
        M: Method + Into<T>,
        T::Response: TryInto<M::Response, Error = ClientError<T::Response>>,
    {
        let (call_id, mut rx) = self.call_base(method).await?;

        let Some(response) = rx.recv().await else {
            // the sender has been dropped because the server closed
            return Err(ClientError::Connection);
        };

        self.channels.lock().await.remove(&call_id);
        // ensure the receiver is dropped after the channel has been removed from the list
        drop(rx);

        response.try_into().map(Response::into_inner)
    }

    pub async fn long_call<M>(
        &self,
        method: M,
    ) -> Result<
        impl Stream<Item = Result<<M::Response as Response>::Inner, ClientError<T::Response>>>,
        ClientError<T::Response>,
    >
    where
        M: LongMethod + Into<T>,
        T::Response: TryInto<Option<M::Response>, Error = ClientError<T::Response>>,
    {
        let (call_id, rx) = self.call_base(method).await?;
        let channels = Arc::clone(&self.channels);

        Ok(stream::unfold(
            (rx, channels, false),
            move |(mut receiver, channels, error_sent)| async move {
                let Some(response) = receiver.recv().await else {
                    // the sender has been dropped because the server closed
                    if error_sent {
                        return None;
                    }
                    return Some((Err(ClientError::Connection), (receiver, channels, true)));
                };
                let response: Option<M::Response> = match response.try_into() {
                    Ok(x) => x,
                    Err(e) => return Some((Err(e), (receiver, channels, error_sent))),
                };

                if response.is_none() {
                    channels.lock().await.remove(&call_id);
                }

                response.map(|x| (Ok(x.into_inner()), (receiver, channels, error_sent)))
            },
        ))
    }

    async fn call_base<M>(
        &self,
        method: M,
    ) -> Result<(u64, mpsc::UnboundedReceiver<T::Response>), ClientError<T::Response>>
    where
        M: Send + Into<T>,
    {
        let call_id = self.next_call_id.fetch_add(1, Ordering::Relaxed);

        let (tx, rx) = mpsc::unbounded_channel();
        self.channels.lock().await.insert(call_id, tx);

        let packet: Clientbound<T> = packet::Clientbound {
            call_id,
            payload: method.into(),
        };
        let result = self.stream.write(packet).await;
        if let Err(e) = result {
            self.channels.lock().await.remove(&call_id);
            return Err(ClientError::Call(e));
        }

        Ok((call_id, rx))
    }
}
