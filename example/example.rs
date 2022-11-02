use async_channel::{bounded, Receiver};
pub use std::alloc;

pub use async_executor::Executor;

async fn dispatch(reciever: Receiver<String>) {
    while let Ok(string) = reciever.recv().await {
        println!("dispatch {}", string);
    }
}

fn main() {
    let ex = Executor::new();
    ex.block_on(async {
        let (sender, receiver) = bounded::<String>(3);
        ex.spawn(dispatch(receiver));
        for i in 0..3 {
            let sender = sender.clone();
            ex.spawn(async move {
                sender.send(format!("begin {i}")).await.unwrap();
                sender.send(format!("end {i}")).await.unwrap();
            });
        }
    });
}
