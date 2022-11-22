use core::{
    ptr,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use crossbeam_queue::ArrayQueue;
use futures_util::Future;
use x86_64::instructions::interrupts::{self, enable_and_hlt};

use super::{Task, TaskId};

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    waker_cache: BTreeMap<TaskId, Waker>,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl Executor {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Executor {
            task_queue: Arc::new(ArrayQueue::new(256)),
            tasks: BTreeMap::new(),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        let task = Task::new(future);
        self.spawn_task(task);
    }

    pub fn spawn_task(&mut self, task: Task) {
        let id = task.id;
        if self.tasks.insert(id, task).is_some() {
            panic!("duplicate taskids")
        }
        self.task_queue.push(id).expect("queue full");
    }

    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue, // task no longer exists
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::make_waker(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn make_waker(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(ptr::null(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
