mod ipc;
mod proxy;
mod state;
mod stats;

use std::path::PathBuf;

fn default_socket_path() -> PathBuf {
    if let Some(home) = dirs_home() {
        home.join("Library/Application Support/KeyProxy/daemon.sock")
    } else {
        PathBuf::from("/tmp/keyproxyd.sock")
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let _ = rustls::crypto::ring::default_provider().install_default();

    let socket = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_socket_path);

    ipc::serve(socket).await
}
