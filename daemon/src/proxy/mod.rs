pub mod cert;
pub mod connect;
pub mod intercept;
pub mod tunnel;

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::proxy::connect::{serve_connection, ConnectCtx};
use crate::state::State;
use crate::stats::LogEntry;

pub struct ProxyHandle {
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
    pub addr: SocketAddr,
}

impl ProxyHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(t) = self.task.take() {
            let _ = t.await;
        }
    }
}

pub async fn start(
    port: u16,
    state: Arc<State>,
    log_tx: mpsc::UnboundedSender<LogEntry>,
) -> Result<ProxyHandle> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse()?;
    let listener = TcpListener::bind(addr).await.with_context(|| format!("bind {addr}"))?;
    let bound = listener.local_addr()?;
    info!("keyproxyd proxy listening on {bound}");

    let (tx, mut rx) = oneshot::channel::<()>();
    let ctx = Arc::new(ConnectCtx { state, log_tx });

    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => {
                    info!("proxy shutdown signal received");
                    return;
                }
                accept = listener.accept() => {
                    match accept {
                        Ok((tcp, _peer)) => {
                            let ctx = ctx.clone();
                            tokio::spawn(async move {
                                tcp.set_nodelay(true).ok();
                                serve_connection(tcp, ctx).await;
                            });
                        }
                        Err(e) => warn!("accept error: {e}"),
                    }
                }
            }
        }
    });

    Ok(ProxyHandle { shutdown: Some(tx), task: Some(task), addr: bound })
}
