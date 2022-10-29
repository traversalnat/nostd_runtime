/// Future yield
use core::pin::Pin;
use core::future::Future;
use core::task::{Context, Poll};

pub struct Yield {
    yielded: bool,
}

impl Yield {
    pub fn new() -> Self {
        Yield { yielded: false }
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.get_mut().yielded = true;
            Poll::Pending
        }
    }
}

pub async fn async_yield() {
    Yield::new().await;
}
