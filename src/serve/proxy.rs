//! Hyper reverse proxy with TLS and SNI.

use anyhow::{Context, Result};
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use hyper_util::server::conn::auto::Builder as HttpBuilder;
use std::net::SocketAddr;
use rustls::pki_types::CertificateDer;
use rustls::server::{ResolvesServerCertUsingSni, ServerConfig};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::config::RoostPaths;

/// Build SNI cert resolver from mappings.
fn build_cert_resolver(
    paths: &RoostPaths,
    mappings: &HashMap<String, u16>,
) -> Result<Arc<ResolvesServerCertUsingSni>> {
    // Get crypto provider (builder triggers install if needed)
    let provider = rustls::ServerConfig::builder().crypto_provider().clone();

    let mut resolver = ResolvesServerCertUsingSni::new();

    // Sort so more specific (longer) domains are added first - SNI does exact match,
    // but we want each domain to have its own cert
    let mut domains: Vec<_> = mappings.keys().collect();
    domains.sort_by(|a, b| b.len().cmp(&a.len()));

    for domain in domains {
        let cert_path = paths.certs_dir.join(format!("{domain}.pem"));
        let key_path = paths.certs_dir.join(format!("{domain}-key.pem"));
        if !cert_path.is_file() || !key_path.is_file() {
            continue;
        }
        let cert_pem = std::fs::read(&cert_path)
            .with_context(|| format!("read cert: {}", cert_path.display()))?;
        let key_pem = std::fs::read(&key_path)
            .with_context(|| format!("read key: {}", key_path.display()))?;

        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .context("parse cert PEM")?;
        let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
            .context("parse key PEM")?
            .context("no private key in file")?;

        let certified_key = CertifiedKey::from_der(certs, key, &provider)
            .with_context(|| format!("load cert for {domain}"))?;
        resolver.add(domain, certified_key)?;
    }

    Ok(Arc::new(resolver))
}

/// Start proxy server.
pub async fn run_proxy(
    paths: &RoostPaths,
    mappings: HashMap<String, u16>,
    port: u16,
) -> Result<()> {
    if mappings.is_empty() {
        anyhow::bail!("no mappings configured; add with 'roost serve config add <domain> <port>'");
    }

    let cert_resolver = build_cert_resolver(paths, &mappings)?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(cert_resolver);
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("Proxy listening on https://0.0.0.0:{}", port);

    // Plain HTTP client for forwarding to localhost backends
    let http_client = Client::builder(TokioExecutor::new())
        .build(HttpConnector::new());

    let mappings = Arc::new(mappings);
    let http_client = Arc::new(http_client);

    loop {
        let (tcp_stream, remote_addr) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();
        let mappings = mappings.clone();
        let client = http_client.clone();

        tokio::spawn(async move {
            let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("TLS handshake failed: {e}");
                    return;
                }
            };

            let service = service_fn({
                let mappings = mappings.clone();
                let client = client.clone();
                let remote_addr = remote_addr;
                move |req: Request<Incoming>| {
                    let mappings = mappings.clone();
                    let client = client.clone();
                    let remote_addr = remote_addr;
                    async move { proxy_request(req, remote_addr, &mappings, &client).await }
                }
            });

            if let Err(e) = HttpBuilder::new(TokioExecutor::new())
                .serve_connection(hyper_util::rt::TokioIo::new(tls_stream), service)
                .await
            {
                eprintln!("connection error: {e}");
            }
        });
    }
}

async fn proxy_request(
    mut req: Request<Incoming>,
    remote_addr: SocketAddr,
    mappings: &HashMap<String, u16>,
    client: &Client<HttpConnector, Incoming>,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    use http_body_util::BodyExt;

    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(':').next().unwrap_or(s).to_string());

    let port = match &host {
        Some(h) => mappings.get(h).copied(),
        None => None,
    };

    let port = match port {
        Some(p) => p,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::from(
                    "Unknown domain; add with 'roost serve config add <domain> <port>'",
                ))
                .unwrap());
        }
    };

    let backend = format!("http://127.0.0.1:{}", port);

    // Add X-Forwarded-* headers
    req.headers_mut()
        .insert("x-forwarded-for", remote_addr.to_string().parse().unwrap());
    req.headers_mut()
        .insert("x-forwarded-proto", "https".parse().unwrap());
    if let Some(ref h) = host {
        req.headers_mut()
            .insert("x-forwarded-host", h.parse().unwrap());
    }

    let uri = format!(
        "{}{}",
        backend,
        req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/")
    );
    *req.uri_mut() = uri.parse().unwrap();

    let response = client.request(req).await?;
    let (parts, body) = response.into_parts();
    let bytes = body.collect().await.context("read body")?.to_bytes();
    Ok(Response::from_parts(parts, Full::from(bytes)))
}
