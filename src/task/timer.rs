use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use futures_util::{task::AtomicWaker, Stream};

static WAKER: AtomicWaker = AtomicWaker::new();
static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn tick() {
    COUNTER.fetch_add(1, Ordering::Relaxed);
    WAKER.wake();
}

pub struct Ticks {
    last: u64,
}

impl Ticks {
    pub fn new() -> Self {
        Self {
            last: COUNTER.load(Ordering::Relaxed),
        }
    }
}

impl Default for Ticks {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for Ticks {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let count = COUNTER.load(Ordering::Acquire);
        if count != self.last {
            self.last = count;
            return Poll::Ready(Some(()));
        }

        WAKER.register(cx.waker());

        let count = COUNTER.load(Ordering::Acquire);
        if count != self.last {
            self.last = count;
            Poll::Ready(Some(()))
        } else {
            Poll::Pending
        }
    }
}
