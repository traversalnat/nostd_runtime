mod async_executor;
mod utils;

// use std to show example
pub use std::alloc;

use async_executor::{spawn, run, block_on, join};
use utils::async_yield;

fn main() {
    let handle_1 = spawn(async {
        loop {
            println!("AAAAAA");
            async_yield().await;
        }
    });

    let handle_2 = spawn(async {
        loop {
            println!("BBBBBB");
            async_yield().await;
        }
    });

    // use block_on instead if you run task with thread in RUNTIME
    run(async {
        join!(handle_1, handle_2);
    });
}
