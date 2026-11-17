use std::{io, time::Duration};

use futures::{AsyncRead, AsyncReadExt};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

/// Presents an asynchronous artifact stream as a blocking [`io::Read`].
pub struct BlockingReader<'a, R> {
    /// Asynchronous stream being read.
    stream: R,
    /// Runtime used to drive the stream.
    runtime: Handle,
    /// Cancels a read that is waiting on the network.
    cancellation: &'a CancellationToken,
    /// Timeout length for waiting for bytes.
    stall_timeout: Duration,
}

impl<'a, R> BlockingReader<'a, R> {
    /// Wraps a stream using the runtime of the calling task.
    pub fn new(stream: R, cancellation: &'a CancellationToken, stall_timeout: Duration) -> Self {
        Self { stream, runtime: Handle::current(), cancellation, stall_timeout }
    }
}

impl<R: AsyncRead + Unpin> io::Read for BlockingReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Self { stream, runtime, cancellation, stall_timeout } = self;
        runtime.block_on(async {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(io::Error::other("crawl cancelled")),
                read = tokio::time::timeout(*stall_timeout, stream.read(buf)) => {
                    read.unwrap_or_else(|_| {
                        Err(io::Error::new(io::ErrorKind::TimedOut, "artifact stream stalled"))
                    })
                }
            }
        })
    }
}
