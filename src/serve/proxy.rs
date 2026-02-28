//! Hyper reverse proxy with TLS and SNI.

use anyhow::{Context, Result};
use http::header::{CONNECTION, UPGRADE};
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::upgrade;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use hyper_util::server::conn::auto::Builder as HttpBuilder;
use rustls::pki_types::CertificateDer;
use rustls::server::{ClientHello, ResolvesServerCert, ServerConfig};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::config::RoostPaths;

const UNSUPPORTED_SNI: &[&str] = &["localhost", "127.0.0.1", "::1"];

#[derive(Clone)]
struct CertResolver {
    certs: HashMap<String, Arc<CertifiedKey>>,
}

impl fmt::Debug for CertResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CertResolver")
            .field("domains", &self.certs.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ResolvesServerCert for CertResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?;
        let key = sni.trim().to_lowercase();
        if key.is_empty() || UNSUPPORTED_SNI.contains(&key.as_str()) {
            return None;
        }
        // SNI sometimes includes port (e.g. "host:443"); use host part for lookup
        let host = key.split(':').next().unwrap_or(&key).trim();
        self.certs.get(host).cloned()
    }
}

fn build_cert_resolver(
    paths: &RoostPaths,
    mappings: &HashMap<String, u16>,
) -> Result<Arc<CertResolver>> {
    let provider = rustls::ServerConfig::builder().crypto_provider().clone();
    let mut certs: HashMap<String, Arc<CertifiedKey>> = HashMap::new();

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

        let certs_der: Vec<CertificateDer<'static>> =
            rustls_pemfile::certs(&mut cert_pem.as_slice())
                .collect::<Result<Vec<_>, _>>()
                .context("parse cert PEM")?;
        let key = rustls_pemfile::private_key(&mut key_pem.as_slice())
            .context("parse key PEM")?
            .context("no private key in file")?;

        let certified_key = Arc::new(
            CertifiedKey::from_der(certs_der, key, &provider)
                .with_context(|| format!("load cert for {domain}"))?,
        );
        certs.insert(domain.to_lowercase(), certified_key);
    }

    if certs.is_empty() {
        anyhow::bail!(
            "no domain certs found (mappings: {}); run 'roost serve config add <domain> <port>' to create certs",
            mappings.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    Ok(Arc::new(CertResolver { certs }))
}

async fn redirect_http_to_https(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    use http_body_util::BodyExt;
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let host = host.split(':').next().unwrap_or(host).trim();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let location = format!("https://{}{}", host, path);
    let _ = req.into_body().collect().await;
    Ok(Response::builder()
        .status(StatusCode::PERMANENT_REDIRECT)
        .header("Location", location)
        .body(Full::from(Bytes::new()))
        .unwrap())
}

pub async fn run_proxy(
    paths: &RoostPaths,
    mappings: HashMap<String, u16>,
    ports: Vec<u16>,
) -> Result<()> {
    if mappings.is_empty() {
        anyhow::bail!("no mappings configured; add with 'roost serve config add <domain> <port>'");
    }
    if ports.is_empty() {
        anyhow::bail!("no ports configured; add with 'roost serve config ports add <port>'");
    }

    let cert_resolver = build_cert_resolver(paths, &mappings)?;
    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(cert_resolver);
    server_config.alpn_protocols = vec![b"http/1.1".to_vec(), b"http/1.0".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
    let http_client = Client::builder(TokioExecutor::new())
        .pool_max_idle_per_host(4)
        .build(HttpConnector::new());
    let mappings = Arc::new(mappings);
    let has_443 = ports.contains(&443);

    for port in &ports {
        if *port == 80 && has_443 {
            let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 80))).await?;
            eprintln!("HTTP redirect listening on http://0.0.0.0:80 (-> https)");
            tokio::spawn(async move {
                loop {
                    let (stream, _) = match listener.accept().await {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("accept error on 80: {e}");
                            continue;
                        }
                    };
                    let service = service_fn(|req: Request<Incoming>| async move {
                        redirect_http_to_https(req).await
                    });
                    if let Err(e) = HttpBuilder::new(TokioExecutor::new())
                        .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                        .await
                    {
                        eprintln!("connection error on 80: {e:#}");
                    }
                }
            });
        } else if *port != 80 {
            let port = *port;
            let mappings = mappings.clone();
            let tls_acceptor = tls_acceptor.clone();
            let http_client = http_client.clone();
            let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
            eprintln!("Proxy listening on https://0.0.0.0:{}", port);
            tokio::spawn(async move {
                loop {
                    let (tcp_stream, remote_addr) = match listener.accept().await {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("accept error on {port}: {e}");
                            continue;
                        }
                    };
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
                                async move {
                                    match proxy_request(req, remote_addr, &mappings, &client).await {
                                        Ok(r) => Ok::<_, anyhow::Error>(r),
                                        Err(e) => {
                                            eprintln!("proxy error: {e:#}");
                                            Ok(Response::builder()
                                                .status(StatusCode::BAD_GATEWAY)
                                                .body(Full::from(format!(
                                                    "Backend error: {e}\n\nIs your app running on the configured port?"
                                                )))
                                                .unwrap())
                                        }
                                    }
                                }
                            }
                        });
                        if let Err(e) = HttpBuilder::new(TokioExecutor::new())
                            .serve_connection_with_upgrades(
                                hyper_util::rt::TokioIo::new(tls_stream),
                                service,
                            )
                            .await
                        {
                            eprintln!("connection error: {e:#}");
                        }
                    });
                }
            });
        }
    }

    std::future::pending::<()>().await;
    #[allow(unreachable_code)]
    Ok(())
}

/// Parse "host" or "host:port" into (domain, optional_port).
fn parse_host(s: &str) -> (String, Option<u16>) {
    let s = s.trim();
    let (host, port) = match s.rfind(':') {
        Some(colon) => {
            let (h, p) = s.split_at(colon);
            let port_str = p.trim_start_matches(':');
            match port_str.parse::<u16>() {
                Ok(port) => (h, Some(port)),
                Err(_) => (s, None),
            }
        }
        None => (s, None),
    };
    (host.to_lowercase(), port)
}

async fn proxy_request(
    mut req: Request<Incoming>,
    remote_addr: SocketAddr,
    mappings: &HashMap<String, u16>,
    client: &Client<HttpConnector, Incoming>,
) -> Result<Response<Full<Bytes>>, anyhow::Error> {
    use http_body_util::BodyExt;

    let host_raw = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .or_else(|| req.uri().authority().map(|a| a.as_str()));

    let (domain, explicit_port) = match host_raw {
        Some(h) => parse_host(h),
        None => {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::from("Missing Host header"))
                .unwrap())
        }
    };

    let port = match explicit_port {
        Some(443) => mappings
            .get(&domain)
            .copied()
            .or_else(|| mappings.iter().find(|(k, _)| k.eq_ignore_ascii_case(&domain)).map(|(_, p)| *p)),
        Some(p) => Some(p),
        None => mappings
            .get(&domain)
            .copied()
            .or_else(|| mappings.iter().find(|(k, _)| k.eq_ignore_ascii_case(&domain)).map(|(_, p)| *p)),
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

    let backend = format!("http://localhost:{}", port);

    req.headers_mut()
        .insert("x-forwarded-for", remote_addr.to_string().parse().unwrap());
    req.headers_mut()
        .insert("x-forwarded-proto", "https".parse().unwrap());
    req.headers_mut()
        .insert("x-forwarded-host", domain.parse().unwrap());

    let uri = format!(
        "{}{}",
        backend,
        req.uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/")
    );
    *req.uri_mut() = uri.parse().unwrap();

    let is_ws_upgrade = req
        .headers()
        .get(CONNECTION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase().contains("upgrade"))
        .unwrap_or(false)
        && req
            .headers()
            .get(UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);

    let server_upgrade = is_ws_upgrade.then(|| upgrade::on(&mut req));

    let mut response = client
        .request(req)
        .await
        .with_context(|| format!("connect to backend {backend}"))?;

    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        let client_upgrade = upgrade::on(&mut response);
        let (parts, _body) = response.into_parts();

        if let Some(server_upgrade) = server_upgrade {
            tokio::spawn(async move {
                match tokio::try_join!(server_upgrade, client_upgrade) {
                    Ok((server_stream, client_stream)) => {
                        let mut server_io = hyper_util::rt::TokioIo::new(server_stream);
                        let mut client_io = hyper_util::rt::TokioIo::new(client_stream);
                        if let Err(e) =
                            tokio::io::copy_bidirectional(&mut server_io, &mut client_io).await
                        {
                            let msg = e.to_string();
                            if !msg.contains("close_notify") && !msg.contains("connection reset") {
                                eprintln!("WebSocket tunnel error: {e}");
                            }
                        }
                    }
                    Err(e) => eprintln!("WebSocket upgrade failed: {e}"),
                }
            });
        }

        return Ok(Response::from_parts(parts, Full::from(Bytes::new())));
    }

    let (parts, body) = response.into_parts();
    let bytes = body.collect().await.context("read body")?.to_bytes();
    Ok(Response::from_parts(parts, Full::from(bytes)))
}
