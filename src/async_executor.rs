extern crate alloc;

use alloc::collections::LinkedList;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use spin::{Lazy, Mutex};

pub use futures::join;

/// A spawned future and its current state.
type Task = async_task::Task<()>;

pub struct JoinHandle<R>(async_task::JoinHandle<R, ()>);

impl<R> Future for JoinHandle<R> {
    type Output = R;

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

/// Runtime definition
pub(crate) struct Runtime {
    pub(crate) task_queue: Queue,
}

impl Runtime {
    pub fn pop_front(&self) -> Option<Task> {
        self.task_queue.lock().pop_front()
    }

    pub fn push_back(&self, task: Task) {
        self.task_queue.lock().push_back(task);
    }
}

static RUNTIME: Lazy<Mutex<Runtime>> = Lazy::new(|| {
    let runtime = Runtime {
        task_queue: Arc::new(Mutex::new(LinkedList::new())),
    };

    // if there is any thread implemention
    // for _ in 0..CPU_NUMS {
    //     thread::spawn(move || loop {
    //         let task = match RUNTIME.lock().pop_front() {
    //             Some(t) => t,
    //             _ => continue,
    //         };
    //         task.run();
    //     });
    // }

    Mutex::new(runtime)
});


/// Spawns a future on the executor.
pub fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = async_task::spawn(
        future,
        |t| {
            RUNTIME.lock().push_back(t);
        },
        (),
    );

    task.schedule();

    JoinHandle(handle)
}

// use when there is any parallel executor
pub fn block_on<F: Future>(mut future: F) -> F::Output {
    let waker = async_task::waker_fn(|| {});

    let mut cx = Context::from_waker(&waker);

    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match Future::poll(future.as_mut(), &mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => {
                // three choices:
                // 1. thread park(unpark in waker), achieve thread first
                // 2. just yield or sleep
                // 3. do nothing
            }
        };
    }
}

// one thread executor
pub fn run<F: Future>(mut future: F) -> F::Output {
    let waker = async_task::waker_fn(|| {});

    let mut cx = Context::from_waker(&waker);

    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match Future::poll(future.as_mut(), &mut cx) {
            Poll::Ready(val) => break val,
            Poll::Pending => {}
        };

        if let Some(task) = RUNTIME.lock().pop_front() {
            task.run();
        } 
    }
}
