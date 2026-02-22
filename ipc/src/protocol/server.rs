use std::{io, pin::pin};

use async_stream::stream;
use futures::{Stream, StreamExt, stream::FuturesUnordered};
use log::error;
use tokio::{
    io::BufWriter,
    net::{
        UnixListener,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    select,
};

use super::{PacketReceiver, StreamPacket};

pub async fn run_server<'a, S, F>(
    server: &'a S,
    listener: UnixListener,
    handle_client: fn(&'a S, PacketReceiver<OwnedReadHalf>, BufWriter<OwnedWriteHalf>) -> F,
) -> io::Result<!>
where
    S: Sync,
    F: Future<Output = ()> + 'a,
{
    let mut client_tasks = FuturesUnordered::new();
    loop {
        select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let (rx, tx) = stream.into_split();
                        client_tasks.push(handle_client(server, PacketReceiver::new(rx), BufWriter::new(tx)));
                    }
                    Err(e) => {
                        error!("Error accepting client: {e}");
                    }
                }
            },
            Some(()) = client_tasks.next(), if !client_tasks.is_empty() => {}
        }
    }
}

pub fn stream_with_id<T>(
    id: u64,
    stream: impl Stream<Item = T>,
) -> impl Stream<Item = (u64, StreamPacket<T, !>)> {
    stream! {
        let mut stream = pin!(stream);

        while let Some(item) = stream.next().await {
            yield (id, StreamPacket::Value(item));
        }

        yield (id, StreamPacket::EndOfStream);
    }
}
