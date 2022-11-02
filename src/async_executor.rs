extern crate alloc;

use alloc::collections::LinkedList;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use spin::Mutex;

pub use futures::join;

/// A spawned future and its current state.
type Task = async_task::Task<()>;

pub struct JoinHandle(async_task::JoinHandle<(), ()>);

impl Future for JoinHandle {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => {
                self.0.waker().wake();
                Poll::Pending
            }
            Poll::Ready(output) => Poll::Ready(output.expect("task failed")),
        }
    }
}

pub(crate) type Queue = Arc<Mutex<LinkedList<Task>>>;
pub(crate) type JoinHandleQueue = Arc<Mutex<LinkedList<JoinHandle>>>;

/// Runtime definition
pub(crate) struct Runtime {
    pub(crate) task_queue: Queue,
    pub(crate) handle_queue: JoinHandleQueue,
}

impl Runtime {
    pub fn task_pop_front(&self) -> Option<Task> {
        self.task_queue.lock().pop_front()
    }

    pub fn handle_pop_front(&self) -> Option<JoinHandle> {
        self.handle_queue.lock().pop_front()
    }

    pub fn handle_push_back(&self, handle: JoinHandle) {
        self.handle_queue.lock().push_back(handle);
    }
}

pub struct Executor {
    runtime: Runtime,
}

impl Executor {
    pub fn new() -> Self {
        let runtime = Runtime {
            task_queue: Arc::new(Mutex::new(LinkedList::new())),
            handle_queue: Arc::new(Mutex::new(LinkedList::new())),
        };
        Self { runtime }
    }

    /// Spawns a future on the executor.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let queue = self.runtime.task_queue.clone();
        let (task, handle) = async_task::spawn(
            future,
            move |t| {
                queue.lock().push_back(t);
            },
            (),
        );

        task.schedule();
        self.runtime.handle_push_back(JoinHandle(handle));
    }

    // one thread executor
    pub fn block_on<F: Future>(&self, mut future: F) {
        let waker = async_task::waker_fn(|| {});

        let mut cx = Context::from_waker(&waker);

        let mut future = unsafe { Pin::new_unchecked(&mut future) };
        let mut main_stop = false;
        loop {
            if !main_stop {
                match Future::poll(future.as_mut(), &mut cx) {
                    Poll::Ready(_) => {
                        main_stop = true;
                    }
                    Poll::Pending => {
                        continue;
                    }
                };
            }

            let len = self.runtime.handle_queue.lock().len();
            if len == 0 {
                break;
            }

            if let Some(task) = self.runtime.task_pop_front() {
                task.run();
            }

            let mut handle = self.runtime.handle_pop_front().unwrap();
            let check_handle = unsafe { Pin::new_unchecked(&mut handle) };
            match Future::poll(check_handle, &mut cx) {
                Poll::Ready(_) => {
                    continue;
                }
                Poll::Pending => {
                    self.runtime.handle_push_back(handle);
                }
            };
        }
    }
}
