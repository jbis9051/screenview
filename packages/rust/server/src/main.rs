use tokio::runtime::Builder;

// TODO: consider raising this number if we end up spawning a lot of blocking tasks
const MAX_BLOCKING_THREADS: usize = 512;

fn main() {
    let rt = Builder::new_multi_thread()
        .enable_io()
        .max_blocking_threads(MAX_BLOCKING_THREADS)
        .build()
        .expect("Failed to build tokio runtime");

    rt.block_on(server_main());
}

async fn server_main() {}
