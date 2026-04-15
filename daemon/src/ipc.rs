use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use crate::proxy::cert::CertStore;
use crate::proxy::{self, ProxyHandle};
use crate::state::{ActiveRule, State};
use crate::stats::LogEntry;

#[derive(Debug, Deserialize)]
pub struct RuleDto {
    pub domain: String,
    pub header_name: String,
    pub credential: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    SetCa { key_pem: String, cert_pem: String },
    SetRules { rules: Vec<RuleDto> },
    Start { port: u16 },
    Stop,
    Ping,
    SubscribeLogs,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: Option<u64>,
    #[serde(flatten)]
    pub cmd: Command,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event<'a> {
    Ok { id: Option<u64> },
    Error { id: Option<u64>, message: String },
    Pong { id: Option<u64> },
    Log(&'a LogEntry),
    Status { active: bool, port: Option<u16> },
}

pub async fn serve(socket_path: std::path::PathBuf) -> Result<()> {
    let _ = std::fs::remove_file(&socket_path);
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("bind unix socket {}", socket_path.display()))?;
    // Restrict to owner only
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&socket_path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&socket_path, perms)?;
    info!("keyproxyd ipc listening at {}", socket_path.display());

    loop {
        let (stream, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream).await {
                warn!("ipc client error: {e}");
            }
        });
    }
}

struct Session {
    state: Arc<State>,
    proxy: Mutex<Option<ProxyHandle>>,
    log_tx: mpsc::UnboundedSender<LogEntry>,
    log_rx: Mutex<Option<mpsc::UnboundedReceiver<LogEntry>>>,
}

async fn handle_client(stream: UnixStream) -> Result<()> {
    let (read, write) = stream.into_split();
    let write = Arc::new(Mutex::new(write));
    let mut reader = BufReader::new(read).lines();

    let (log_tx, log_rx) = mpsc::unbounded_channel::<LogEntry>();
    let session = Arc::new(Session {
        state: Arc::new(State::new()),
        proxy: Mutex::new(None),
        log_tx,
        log_rx: Mutex::new(Some(log_rx)),
    });

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                send(&write, &Event::Error { id: None, message: format!("parse: {e}") }).await?;
                continue;
            }
        };
        handle_command(session.clone(), write.clone(), req).await;
    }

    // Client disconnected — fail closed: stop proxy.
    if let Some(h) = session.proxy.lock().await.take() {
        h.shutdown().await;
    }
    Ok(())
}

async fn handle_command(
    session: Arc<Session>,
    write: Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>,
    req: Request,
) {
    let id = req.id;
    let result: Result<()> = async {
        match req.cmd {
            Command::SetCa { key_pem, cert_pem } => {
                let store = CertStore::from_pem(&key_pem, &cert_pem)?;
                session.state.set_ca(Arc::new(store));
            }
            Command::SetRules { rules } => {
                let mapped: Vec<(String, ActiveRule)> = rules
                    .into_iter()
                    .map(|r| {
                        (
                            r.domain,
                            ActiveRule {
                                header_name: r.header_name,
                                credential: r.credential,
                            },
                        )
                    })
                    .collect();
                session.state.set_rules(mapped);
            }
            Command::Start { port } => {
                let mut guard = session.proxy.lock().await;
                if guard.is_none() {
                    let handle =
                        proxy::start(port, session.state.clone(), session.log_tx.clone()).await?;
                    *guard = Some(handle);
                }
            }
            Command::Stop => {
                if let Some(h) = session.proxy.lock().await.take() {
                    h.shutdown().await;
                }
            }
            Command::Ping => {
                send(&write, &Event::Pong { id }).await?;
                return Ok(());
            }
            Command::SubscribeLogs => {
                let mut rx_slot = session.log_rx.lock().await;
                let Some(mut rx) = rx_slot.take() else {
                    anyhow::bail!("logs already subscribed");
                };
                drop(rx_slot);
                let write = write.clone();
                tokio::spawn(async move {
                    while let Some(entry) = rx.recv().await {
                        if send(&write, &Event::Log(&entry)).await.is_err() {
                            break;
                        }
                    }
                });
            }
        }
        Ok(())
    }
    .await;

    match result {
        Ok(()) => {
            let _ = send(&write, &Event::Ok { id }).await;
        }
        Err(e) => {
            let _ = send(
                &write,
                &Event::Error { id, message: e.to_string() },
            )
            .await;
        }
    }
}

async fn send<'a>(
    write: &Arc<Mutex<tokio::net::unix::OwnedWriteHalf>>,
    ev: &Event<'a>,
) -> Result<()> {
    let mut line = serde_json::to_string(ev)?;
    line.push('\n');
    let mut g = write.lock().await;
    g.write_all(line.as_bytes()).await?;
    g.flush().await?;
    Ok(())
}
