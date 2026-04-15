use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::warn;

use crate::proxy::{intercept, tunnel};
use crate::state::State;
use crate::stats::LogEntry;

pub struct ConnectCtx {
    pub state: Arc<State>,
    pub log_tx: mpsc::UnboundedSender<LogEntry>,
}

pub async fn serve_connection(tcp: TcpStream, ctx: Arc<ConnectCtx>) {
    let service = service_fn(move |req: Request<Incoming>| {
        let ctx = ctx.clone();
        async move { handle(req, ctx).await }
    });
    if let Err(e) = hyper::server::conn::http1::Builder::new()
        .preserve_header_case(true)
        .serve_connection(TokioIo::new(tcp), service)
        .with_upgrades()
        .await
    {
        warn!("proxy connection error: {e}");
    }
}

async fn handle(
    req: Request<Incoming>,
    ctx: Arc<ConnectCtx>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    if req.method() != Method::CONNECT {
        return Ok(text_resp(
            StatusCode::METHOD_NOT_ALLOWED,
            "KeyProxy only accepts CONNECT (HTTPS proxy)",
        ));
    }

    let Some(authority) = req.uri().authority().cloned() else {
        return Ok(text_resp(StatusCode::BAD_REQUEST, "CONNECT missing authority"));
    };
    let host = authority.host().to_string();
    let port = authority.port_u16().unwrap_or(443);
    let target = format!("{host}:{port}");

    let rule = ctx.state.rule_for(&host);
    let ca = ctx.state.ca();

    tokio::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                match (rule, ca) {
                    (Some(rule), Some(ca)) => {
                        if let Err(e) = intercept::intercept(
                            upgraded,
                            host.clone(),
                            rule,
                            ca,
                            ctx.log_tx.clone(),
                        )
                        .await
                        {
                            warn!("intercept error for {host}: {e}");
                        }
                    }
                    _ => {
                        if let Err(e) = tunnel::tunnel(upgraded, target.clone()).await {
                            warn!("tunnel error for {target}: {e}");
                        }
                        let _ = ctx.log_tx.send(LogEntry {
                            timestamp: chrono::Utc::now(),
                            domain: host,
                            status: None,
                            latency_ms: 0,
                            intercepted: false,
                            error: None,
                        });
                    }
                }
            }
            Err(e) => warn!("upgrade failed: {e}"),
        }
    });

    Ok(Response::new(empty()))
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new().map_err(|never| match never {}).boxed()
}

fn text_resp(status: StatusCode, msg: &'static str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = http_body_util::Full::new(Bytes::from(msg))
        .map_err(|never| match never {})
        .boxed();
    Response::builder().status(status).body(body).unwrap()
}
