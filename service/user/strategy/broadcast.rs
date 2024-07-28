use arrayvec::ArrayVec;
use eyre::Result;
use kanal::{AsyncReceiver, AsyncSender};
use parking_lot::RwLock;
use std::sync::Arc;

const CAPACITY: usize = 16;
// TODO: move to utils/
// NOTE: approximately 250+ buffer size is needed
/// broadcast an event to multiple receivers (Clone requires Clone)
/// only broadcast when the subscriber id is stored in is_open
struct AsyncBroadcasterInner<T: Clone> {
    /// only broadcast when the subscriber id is stored in is_open. it should be stored by default
    subscribers: ArrayVec<(u8, AsyncSender<T>), CAPACITY>,
    buffer_size: usize,
    broadcaster_id: u8,
}
impl<T: Clone> AsyncBroadcasterInner<T> {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer_size,
            subscribers: ArrayVec::new(),
            broadcaster_id: 0,
        }
    }
    /// subscribe to the broadcaster
    pub fn subscribe(&mut self) -> AsyncReceiver<T> {
        let (tx, rx) = kanal::bounded_async::<T>(self.buffer_size);
        let id = self.broadcaster_id;
        self.subscribers.push((id, tx));
        self.broadcaster_id += 1;
        rx
    }

    /// send to all subscribers that has the status open
    pub fn broadcast(&self, data: T) -> Result<()> {
        let mut fail_by_full = ArrayVec::<u8, CAPACITY>::new();
        let mut fail_by_gone = ArrayVec::<u8, CAPACITY>::new();
        // broadcast should be done for all channels no matter if it failed one or not

        for (i, subscriber) in self.subscribers.iter() {
            match subscriber.try_send(data.clone()) {
                Ok(true) => {}
                Ok(false) => {
                    fail_by_full.push(*i);
                }
                Err(_) => {
                    fail_by_gone.push(*i);
                }
            }
        }
        if fail_by_gone.len() + fail_by_full.len() == 0 {
            Ok(())
        } else {
            eyre::bail!("broadcast fail (full: {:?}, conn: {:?})", fail_by_full, fail_by_gone)
        }
    }
}

#[derive(Clone)]
pub struct AsyncBroadcaster<T: Clone> {
    inner: Arc<RwLock<AsyncBroadcasterInner<T>>>,
}
impl<T: Clone> AsyncBroadcaster<T> {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(AsyncBroadcasterInner::new(buffer_size))),
        }
    }
    pub fn subscribe(&self) -> AsyncReceiver<T> {
        self.inner.write().subscribe()
    }
    pub fn broadcast(&self, data: T) -> Result<()> {
        self.inner.read().broadcast(data)
    }
}
