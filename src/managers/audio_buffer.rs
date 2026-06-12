use std::fmt;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};

/// Append-only audio buffer shared between the HLS download task (writer) and
/// the rodio decoder (reader). Playback can start as soon as the first segment
/// has been demuxed; readers block until the bytes they need arrive.
pub struct SharedAudioBuffer {
    inner: Mutex<Inner>,
    data_available: Condvar,
    // Wakes a prefetch download task waiting for activation
    activation: tokio::sync::Notify,
    total_segments: u32,
}

struct Inner {
    data: Vec<u8>,
    finished: bool,
    cancelled: bool,
    // When false, the download task pauses after the prefetch window until
    // the buffer becomes the playing track (or is cancelled)
    activated: bool,
    completed_segments: u32,
}

impl SharedAudioBuffer {
    pub fn new(total_segments: u32, capacity_hint: usize, start_active: bool) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                data: Vec::with_capacity(capacity_hint),
                finished: false,
                cancelled: false,
                activated: start_active,
                completed_segments: 0,
            }),
            data_available: Condvar::new(),
            activation: tokio::sync::Notify::new(),
            total_segments,
        })
    }

    /// Allow a prefetch download to continue past its initial window.
    /// No-op for buffers that started active.
    pub fn activate(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.activated = true;
        drop(inner);
        self.activation.notify_waiters();
    }

    /// Wait until the buffer is activated or cancelled.
    pub async fn wait_until_active(&self) {
        loop {
            // Register for notification before checking state, so an
            // activate()/cancel() between the check and the await isn't lost
            let notified = self.activation.notified();
            {
                let inner = self.inner.lock().unwrap();
                if inner.activated || inner.cancelled {
                    return;
                }
            }
            notified.await;
        }
    }

    /// Append the demuxed audio for one completed segment. `data` may be empty
    /// when the demuxer is holding back an incomplete frame.
    pub fn append_segment(&self, data: &[u8]) {
        let mut inner = self.inner.lock().unwrap();
        inner.data.extend_from_slice(data);
        inner.completed_segments += 1;
        drop(inner);
        self.data_available.notify_all();
    }

    /// Append trailing bytes that don't correspond to a segment (demuxer flush).
    pub fn append(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let mut inner = self.inner.lock().unwrap();
        inner.data.extend_from_slice(data);
        drop(inner);
        self.data_available.notify_all();
    }

    /// Mark the stream complete and wake any blocked readers.
    pub fn finish(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.finished = true;
        drop(inner);
        self.data_available.notify_all();
    }

    /// Stop the download and unblock readers. Already-buffered data remains
    /// readable, so a finished track can still be replayed.
    pub fn cancel(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.cancelled = true;
        inner.finished = true;
        drop(inner);
        self.data_available.notify_all();
        self.activation.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.lock().unwrap().cancelled
    }

    /// Number of bytes buffered so far.
    pub fn available(&self) -> usize {
        self.inner.lock().unwrap().data.len()
    }

    /// Estimated final byte size, extrapolated from segment download progress.
    /// Exact once the download has finished.
    pub fn estimated_total(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        if inner.finished
            || inner.completed_segments == 0
            || inner.completed_segments >= self.total_segments
        {
            inner.data.len()
        } else {
            inner.data.len() * self.total_segments as usize / inner.completed_segments as usize
        }
    }

    /// Run `f` against the currently buffered bytes (e.g. ADTS frame scans).
    pub fn with_data<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        let inner = self.inner.lock().unwrap();
        f(&inner.data)
    }

    /// Create a reader starting at the given byte offset.
    pub fn reader_at(self: &Arc<Self>, offset: usize) -> StreamReader {
        StreamReader {
            buffer: Arc::clone(self),
            pos: offset as u64,
        }
    }
}

impl fmt::Debug for SharedAudioBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        f.debug_struct("SharedAudioBuffer")
            .field("len", &inner.data.len())
            .field("finished", &inner.finished)
            .field("cancelled", &inner.cancelled)
            .field(
                "segments",
                &format_args!("{}/{}", inner.completed_segments, self.total_segments),
            )
            .finish()
    }
}

/// `Read + Seek` view over a `SharedAudioBuffer` for rodio's decoder. Reads
/// past the buffered end block until more data arrives or the stream finishes.
pub struct StreamReader {
    buffer: Arc<SharedAudioBuffer>,
    pos: u64,
}

impl Read for StreamReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut inner = self.buffer.inner.lock().unwrap();
        loop {
            let available = inner.data.len() as u64;
            if self.pos < available {
                let start = self.pos as usize;
                let n = buf.len().min(inner.data.len() - start);
                buf[..n].copy_from_slice(&inner.data[start..start + n]);
                self.pos += n as u64;
                return Ok(n);
            }
            if inner.finished {
                return Ok(0);
            }
            inner = self.buffer.data_available.wait(inner).unwrap();
        }
    }
}

impl Seek for StreamReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(delta) => self.pos as i64 + delta,
            SeekFrom::End(delta) => self.buffer.available() as i64 + delta,
        };
        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before start of stream",
            ));
        }
        self.pos = new_pos as u64;
        Ok(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::time::Duration;

    #[test]
    fn read_blocks_until_data_arrives_and_eof_after_finish() {
        let buffer = SharedAudioBuffer::new(2, 0, true);
        let mut reader = buffer.reader_at(0);

        let writer = {
            let buffer = Arc::clone(&buffer);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                buffer.append_segment(&[1, 2, 3]);
                std::thread::sleep(Duration::from_millis(50));
                buffer.append_segment(&[4, 5]);
                buffer.finish();
            })
        };

        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        writer.join().unwrap();

        assert_eq!(out, vec![1, 2, 3, 4, 5]);
        assert_eq!(buffer.available(), 5);
        assert_eq!(buffer.estimated_total(), 5);
    }

    #[test]
    fn cancel_unblocks_reader_and_keeps_data() {
        let buffer = SharedAudioBuffer::new(10, 0, true);
        buffer.append_segment(&[9, 9]);
        let mut reader = buffer.reader_at(2); // positioned past available data

        let canceller = {
            let buffer = Arc::clone(&buffer);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                buffer.cancel();
            })
        };

        let mut buf = [0u8; 4];
        let n = reader.read(&mut buf).unwrap();
        canceller.join().unwrap();

        assert_eq!(n, 0, "cancelled stream must EOF, not block");
        assert!(buffer.is_cancelled());
        assert_eq!(buffer.available(), 2, "buffered data survives cancel");
    }

    #[tokio::test]
    async fn wait_until_active_resumes_on_activate_and_on_cancel() {
        // activate() releases the gate
        let buffer = SharedAudioBuffer::new(4, 0, false);
        let waiter = tokio::spawn({
            let buffer = Arc::clone(&buffer);
            async move { buffer.wait_until_active().await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!waiter.is_finished(), "must wait while inactive");
        buffer.activate();
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("activate must release the gate")
            .unwrap();

        // cancel() releases the gate too
        let buffer = SharedAudioBuffer::new(4, 0, false);
        let waiter = tokio::spawn({
            let buffer = Arc::clone(&buffer);
            async move { buffer.wait_until_active().await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        buffer.cancel();
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("cancel must release the gate")
            .unwrap();
    }

    #[test]
    fn estimated_total_extrapolates_from_segment_progress() {
        let buffer = SharedAudioBuffer::new(4, 0, true);
        buffer.append_segment(&[0u8; 100]);
        assert_eq!(buffer.estimated_total(), 400);
        buffer.append_segment(&[0u8; 300]);
        assert_eq!(buffer.estimated_total(), 800);
        buffer.finish();
        assert_eq!(buffer.estimated_total(), 400);
    }
}
