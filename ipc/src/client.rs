use crate::{
    errors::ClientError,
    packet::{self, Clientbound},
    rw::{Read, Write},
    AnyCall, LongMethod, Method,
};
use futures::Stream;
use log::{error, warn};
use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    io,
    marker::PhantomData,
    os::unix::net::{SocketAddr, UnixStream as StdUnixStream},
    pin::{pin, Pin},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{unix::OwnedWriteHalf, UnixStream},
    spawn,
    sync::{mpsc, Mutex},
};

#[derive(Debug, Clone)]
pub struct Connection<T: AnyCall, TX: AsyncWriteExt + Unpin + Send> {
    stream: Arc<Mutex<TX>>,
    next_call_id: Arc<AtomicU64>,
    channels: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<T::Response>>>>,
}

#[derive(Debug)]
pub struct ResponseStream<T: AnyCall, M: LongMethod> {
    call_id: u64,
    channels: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<T::Response>>>>,
    receiver: mpsc::UnboundedReceiver<T::Response>,
    phantom: PhantomData<M>,
}

impl<T: AnyCall> Connection<T, OwnedWriteHalf>
where
    anyhow::Error: From<<T as Write>::Error> + From<<T::Response as Read>::Error>,
{
    pub fn from_unix_address(address: &SocketAddr) -> io::Result<Self> {
        let stream = StdUnixStream::connect_addr(address)?;
        stream.set_nonblocking(true)?;
        let (rx, tx) = UnixStream::from_std(stream)?.into_split();

        Ok(Self::new(rx, tx))
    }
}

impl<T: AnyCall, TX: AsyncWriteExt + Unpin + Send> Connection<T, TX>
where
    anyhow::Error: From<<T as Write>::Error> + From<<T::Response as Read>::Error>,
{
    pub fn new<RX: AsyncReadExt + Unpin + Send + 'static>(mut rx: RX, tx: TX) -> Self {
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
                        error!("Error while receiving response: {e}");
                        break;
                    }
                };
            }
        });

        Self {
            stream: Arc::new(Mutex::new(tx)),
            next_call_id: Arc::new(AtomicU64::new(0)),
            channels,
        }
    }

    pub async fn call<M>(&self, method: M) -> Result<M::Response, ClientError<T::Response>>
    where
        M: Method + Into<T>,
        T::Response: TryInto<M::Response, Error = ClientError<T::Response>>,
    {
        let (call_id, mut rx) = self.call_base(method).await?;

        // unwrap: the sender is dropped on the next line, after this call
        let response = rx.recv().await.unwrap();

        self.channels.lock().await.remove(&call_id);
        // ensure the receiver is dropped after the channel has been removed from the list
        drop(rx);

        response.try_into()
    }

    pub async fn long_call<M>(
        &self,
        method: M,
    ) -> Result<ResponseStream<T, M>, ClientError<T::Response>>
    where
        M: LongMethod + Into<T>,
    {
        let (call_id, rx) = self.call_base(method).await?;

        Ok(ResponseStream {
            call_id,
            channels: Arc::clone(&self.channels),
            receiver: rx,
            phantom: PhantomData,
        })
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
        let result = packet.write(&mut *self.stream.lock().await).await;
        if let Err(e) = result {
            self.channels.lock().await.remove(&call_id);
            return Err(ClientError::Call(e));
        }

        Ok((call_id, rx))
    }
}

impl<T: AnyCall, M: LongMethod + Unpin> Stream for ResponseStream<T, M>
where
    T::Response: TryInto<Option<M::Response>, Error = ClientError<T::Response>>,
{
    type Item = Result<M::Response, ClientError<T::Response>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let future = pin!(async move {
            // unwrap: the sender is dropped after the end of the stream
            let response = self.receiver.recv().await.unwrap();
            let response: Option<M::Response> = match response.try_into() {
                Ok(x) => x,
                Err(e) => return Some(Err(e)),
            };

            if response.is_none() {
                self.channels.lock().await.remove(&self.call_id);
            }

            response.map(Ok)
        });
        future.poll(cx)
    }
}
