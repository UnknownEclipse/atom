use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};

use alloc::boxed::Box;
use futures_util::Future;

pub mod executor;
pub mod keyboard;
pub mod timer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn get() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Task {
    id: TaskId,
    task: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        Self {
            id: TaskId::get(),
            task: Box::pin(future),
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.task.as_mut().poll(cx)
    }
}
