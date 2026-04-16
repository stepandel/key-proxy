mod ipc;
mod network;
mod proxy;
mod state;
mod stats;

use std::path::PathBuf;
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

fn default_socket_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        home.join("Library/Application Support/KeyProxy/daemon.sock")
    } else {
        PathBuf::from("/tmp/keyproxyd.sock")
    }
}

fn print_usage() {
    eprintln!(
        "keyproxyd — credential-injecting HTTPS proxy daemon

Usage:
  keyproxyd [SOCKET_PATH]        run daemon, default socket at
                                 ~/Library/Application Support/KeyProxy/daemon.sock
  keyproxyd --unset              disable system HTTPS proxy on all interfaces and exit
                                 (panic / recovery command — always safe to run)
  keyproxyd --help               show this message"
    );
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return Ok(());
    }
    if args.iter().any(|a| a == "--unset") {
        info!("disabling system HTTP/HTTPS proxy on all network services");
        network::disable_all();
        return Ok(());
    }

    let socket = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(default_socket_path);

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(run(socket))
}

async fn run(socket: PathBuf) -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let serve = ipc::serve(socket);

    let shutdown = async {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => info!("SIGTERM received"),
            _ = sigint.recv() => info!("SIGINT received"),
        }
    };

    let result = tokio::select! {
        res = serve => res,
        _ = shutdown => Ok(()),
    };

    // Belt-and-braces: always clear the system proxy on daemon exit. If the
    // Swift side is the one that set it, this is a no-op (it already cleared);
    // if the daemon is orphaned or crashes, this prevents a dead proxy from
    // blackholing the user's internet.
    info!("clearing system proxy before exit");
    network::disable_all();

    result
}
