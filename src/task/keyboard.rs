use core::{
    pin::Pin,
    task::{Context, Poll},
};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{task::AtomicWaker, Stream, StreamExt};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use tracing::warn;

use crate::print;

static SCANCODE_QUEUE: ScancodeQueue = ScancodeQueue::new();

pub struct ScancodeStream(());

impl ScancodeStream {
    pub fn new() -> Option<Self> {
        SCANCODE_QUEUE
            .queue
            .try_init_once(|| ArrayQueue::new(128))
            .map(|_| Self(()))
            .ok()
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.queue.get().unwrap();

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        SCANCODE_QUEUE.waker.register(cx.waker());
        if let Some(scancode) = queue.pop() {
            SCANCODE_QUEUE.waker.take();
            Poll::Ready(Some(scancode))
        } else {
            Poll::Pending
        }
    }
}

pub fn add_scancode(scancode: u8) {
    if let Some(queue) = SCANCODE_QUEUE.queue.get() {
        if queue.push(scancode).is_ok() {
            SCANCODE_QUEUE.waker.wake();
        } else {
            warn!("scancode queue full; dropping incoming keyboard codes")
        }
    } else {
        warn!("scancode queue not initialized; dropping incoming keyboard codes");
    }
}

#[derive(Debug)]
struct ScancodeQueue {
    queue: OnceCell<ArrayQueue<u8>>,
    waker: AtomicWaker,
}

impl ScancodeQueue {
    pub const fn new() -> Self {
        Self {
            queue: OnceCell::uninit(),
            waker: AtomicWaker::new(),
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new().expect("failed to get scancode stream");
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
