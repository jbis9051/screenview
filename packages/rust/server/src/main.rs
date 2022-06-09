mod config;
mod logging;
pub use config::Config;

use log::{error, info};
use std::future::Future;
use tokio::{runtime::Builder, sync::oneshot::channel};

pub type HashMap<K, V> = hashbrown::HashMap<K, V, ahash::RandomState>;

fn main() {
    let config = match Config::get_or_try_init() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to initialize config: {error}");
            return;
        }
    };

    if let Err(error) = logging::init_logger("server") {
        eprintln!("Failed to initialize logger: {error}");
        return;
    }

    info!("Initializing runtime...");

    let rt = match Builder::new_multi_thread()
        .enable_io()
        .max_blocking_threads(config.max_blocking_threads)
        .build()
    {
        Ok(rt) => rt,
        Err(error) => {
            error!("Failed to build runtime: {error}");
            return;
        }
    };

    info!("Runtime initialized.");
    info!("Starting server...");

    rt.spawn(server_main());
    rt.block_on(shutdown_hook());

    drop(rt);
    logging::cleanup();
    println!();
}

pub fn shutdown_hook() -> impl Future<Output = ()> {
    let (tx, rx) = channel::<()>();

    let set_handler_result = ctrlc::set_handler({
        let mut tx = Some(tx);
        move || {
            if let Some(tx) = tx.take() {
                let _ = tx.send(());
            }
        }
    });

    if let Err(error) = set_handler_result {
        error!("Failed to set shutdown hook: {error}");
        // Sender has been dropped, so the returned future exits immediately
    }

    async move {
        let _ = rx.await;
    }
}

async fn server_main() {
    info!("server_main");
}
