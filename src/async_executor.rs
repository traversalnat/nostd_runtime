extern crate alloc;

use alloc::collections::LinkedList;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use spin::Mutex;

pub use futures::join;

type PinBoxFuture = Pin<Box<dyn Future<Output = ()>>>;

pub(crate) type Queue = Arc<Mutex<LinkedList<PinBoxFuture>>>;

/// Runtime definition
pub(crate) struct Runtime {
    pub(crate) task_queue: Queue,
}

impl Runtime {
    pub fn task_pop_front(&self) -> Option<PinBoxFuture> {
        self.task_queue.lock().pop_front()
    }

    pub fn task_push_back(&self, task: PinBoxFuture) {
        self.task_queue.lock().push_back(task)
    }
}

pub struct Executor {
    runtime: Runtime,
}

impl Executor {
    pub fn new() -> Self {
        let runtime = Runtime {
            task_queue: Arc::new(Mutex::new(LinkedList::new())),
        };
        Self { runtime }
    }

    /// Spawns a future on the executor.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.task_push_back(Box::pin(future));
    }

    // one thread executor
    pub fn block_on<F: Future>(&self, mut future: F) {
        let waker = async_task::waker_fn(|| {});

        let mut cx = Context::from_waker(&waker);

        let mut future = unsafe { Pin::new_unchecked(&mut future) };
        let mut main_stopped = false;
        loop {
            if !main_stopped {
                match Future::poll(future.as_mut(), &mut cx) {
                    Poll::Ready(_) => {
                        main_stopped = true;
                    }
                    Poll::Pending => {}
                };
            }

            while let Some(mut handle) = self.runtime.task_pop_front() {
                let check_handle = unsafe { Pin::new_unchecked(&mut handle) };
                match Future::poll(check_handle, &mut cx) {
                    Poll::Ready(_) => {
                        continue;
                    }
                    Poll::Pending => {
                        self.runtime.task_push_back(handle);
                    }
                };
            }

            if main_stopped {
                break;
            }
        }
    }
}
