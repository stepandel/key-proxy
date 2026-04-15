use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpStream;
use tracing::warn;

use crate::config::ConfigStore;
use crate::proxy::cert::CertStore;
use crate::proxy::{intercept, tunnel};
use crate::stats::{LogEntry, Stats};

pub struct ConnectCtx {
    pub config: Arc<ConfigStore>,
    pub certs: Arc<CertStore>,
    pub stats: Arc<Stats>,
}

pub async fn serve_connection(tcp: TcpStream, ctx: Arc<ConnectCtx>) {
    let io = TokioIo::new(tcp);
    let service = service_fn(move |req: Request<Incoming>| {
        let ctx = ctx.clone();
        async move { handle(req, ctx).await }
    });
    if let Err(e) = hyper::server::conn::http1::Builder::new()
        .preserve_header_case(true)
        .serve_connection(io, service)
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

    let rule = ctx.config.enabled_rule_for(&host);

    tokio::spawn(async move {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                if let Some(rule) = rule {
                    if let Err(e) =
                        intercept::intercept(upgraded, rule, ctx.certs.clone(), ctx.stats.clone())
                            .await
                    {
                        warn!("intercept error for {host}: {e}");
                    }
                } else {
                    if let Err(e) = tunnel::tunnel(upgraded, target.clone()).await {
                        warn!("tunnel error for {target}: {e}");
                    }
                    ctx.stats.record(LogEntry {
                        timestamp: chrono::Utc::now(),
                        domain: host,
                        status: None,
                        latency_ms: 0,
                        intercepted: false,
                        error: None,
                    });
                }
            }
            Err(e) => warn!("upgrade failed: {e}"),
        }
    });

    Ok(Response::new(empty()))
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn text_resp(status: StatusCode, msg: &'static str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = http_body_util::Full::new(Bytes::from(msg))
        .map_err(|never| match never {})
        .boxed();
    Response::builder().status(status).body(body).unwrap()
}
