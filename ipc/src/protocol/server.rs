use std::{io, pin::pin, time::Duration};

use async_stream::stream;
use futures::{Stream, StreamExt};
use log::error;
use tokio::{
    io::BufWriter,
    net::{
        UnixListener,
        unix::{OwnedReadHalf, OwnedWriteHalf},
    },
    select, spawn,
    task::JoinHandle,
    time::{Instant, sleep},
};

use super::{PacketReceiver, StreamPacket};

#[derive(Default)]
struct CancelGuard {
    tasks: Vec<JoinHandle<()>>,
}

impl CancelGuard {
    pub fn add(&mut self, task: JoinHandle<()>) {
        self.tasks.push(task);
    }

    pub fn cleanup(&mut self) {
        self.tasks.retain(|task| !task.is_finished());
    }
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

const TIMEOUT: Duration = Duration::from_mins(1);

pub async fn run_server<S, F>(
    server: S,
    listener: UnixListener,
    handle_client: fn(S, PacketReceiver<OwnedReadHalf>, BufWriter<OwnedWriteHalf>) -> F,
) -> io::Result<!>
where
    S: Clone + Send + Sync + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    let mut guard = CancelGuard::default();

    let timer = sleep(TIMEOUT);
    let mut timer = pin!(timer);

    loop {
        select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let (rx, tx) = stream.into_split();
                        let task = spawn(handle_client(server.clone(), PacketReceiver::new(rx), BufWriter::new(tx)));
                        guard.add(task);
                    }
                    Err(e) => {
                        error!("Error accepting client: {e}");
                    }
                }
            },
            () = &mut timer => {
                // Timeout reached, cleanup dead tasks
                guard.cleanup();
                timer.as_mut().reset(Instant::now() + TIMEOUT);
            }
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
