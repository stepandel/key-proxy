use anyhow::{anyhow, Context, Result};
use http_body_util::combinators::BoxBody;
use http_body_util::BodyExt;
use hyper::body::{Bytes, Incoming};
use hyper::header::{HeaderName, HeaderValue, HOST};
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use once_cell::sync::Lazy;
use rustls::pki_types::ServerName;
use rustls::ClientConfig;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tracing::warn;

use crate::config::Rule;
use crate::keychain;
use crate::proxy::cert::CertStore;
use crate::stats::{LogEntry, Stats};

static CLIENT_CONFIG: Lazy<Arc<ClientConfig>> = Lazy::new(|| {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Arc::new(cfg)
});

pub async fn intercept(
    upgraded: Upgraded,
    rule: Rule,
    certs: Arc<CertStore>,
    stats: Arc<Stats>,
) -> Result<()> {
    let server_config = certs.server_config_for(&rule.domain)?;
    let acceptor = TlsAcceptor::from(server_config);
    let io = TokioIo::new(upgraded);
    let tls = acceptor
        .accept(io)
        .await
        .context("TLS accept from client")?;

    let rule = Arc::new(rule);
    let stats = stats.clone();

    let service = service_fn(move |req: Request<Incoming>| {
        let rule = rule.clone();
        let stats = stats.clone();
        async move { handle_request(req, rule, stats).await }
    });

    let tls_io = TokioIo::new(tls);
    if let Err(e) = hyper::server::conn::http1::Builder::new()
        .preserve_header_case(true)
        .serve_connection(tls_io, service)
        .await
    {
        warn!("intercept connection error: {e}");
    }
    Ok(())
}

async fn handle_request(
    mut req: Request<Incoming>,
    rule: Arc<Rule>,
    stats: Arc<Stats>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let started = Instant::now();
    let domain = rule.domain.clone();

    // Strip hop-by-hop and ensure Host header matches
    remove_hop_by_hop(&mut req);
    if let Ok(hv) = HeaderValue::from_str(&domain) {
        req.headers_mut().insert(HOST, hv);
    }

    // Inject credential
    match keychain::get_credential(&domain) {
        Some(value) => match (HeaderName::try_from(&rule.header_name), HeaderValue::from_str(&value)) {
            (Ok(name), Ok(val)) => {
                req.headers_mut().insert(name, val);
            }
            _ => {
                warn!("invalid header name or credential for {domain}");
            }
        },
        None => {
            warn!("no credential in keychain for {domain}");
        }
    }

    match forward(req, &domain).await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            stats.record(LogEntry {
                timestamp: chrono::Utc::now(),
                domain: domain.clone(),
                status: Some(status),
                latency_ms: started.elapsed().as_millis() as u64,
                intercepted: true,
                error: None,
            });
            Ok(resp)
        }
        Err(e) => {
            stats.record(LogEntry {
                timestamp: chrono::Utc::now(),
                domain,
                status: None,
                latency_ms: started.elapsed().as_millis() as u64,
                intercepted: true,
                error: Some(e.to_string()),
            });
            let body = http_body_util::Full::new(Bytes::from(format!("keyproxy upstream error: {e}")))
                .map_err(|never| match never {})
                .boxed();
            Ok(Response::builder()
                .status(502)
                .body(body)
                .unwrap())
        }
    }
}

async fn forward(
    req: Request<Incoming>,
    domain: &str,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    let addr = format!("{domain}:443");
    let tcp = TcpStream::connect(&addr).await.context("tcp connect upstream")?;
    tcp.set_nodelay(true).ok();
    let connector = TlsConnector::from(CLIENT_CONFIG.clone());
    let server_name = ServerName::try_from(domain.to_string())
        .map_err(|e| anyhow!("invalid server name: {e}"))?;
    let tls = connector
        .connect(server_name, tcp)
        .await
        .context("TLS connect upstream")?;

    let io = TokioIo::new(tls);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
        .await
        .context("http1 handshake upstream")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            warn!("upstream connection error: {e}");
        }
    });

    let resp = sender
        .send_request(req)
        .await
        .context("send upstream request")?;
    let (parts, body) = resp.into_parts();
    let body = body.boxed();
    Ok(Response::from_parts(parts, body))
}

fn remove_hop_by_hop(req: &mut Request<Incoming>) {
    const HOP_BY_HOP: &[&str] = &[
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
    ];
    let headers = req.headers_mut();
    for h in HOP_BY_HOP {
        headers.remove(*h);
    }
}
