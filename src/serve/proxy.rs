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
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::config::RoostPaths;

/// TLS handshake record type (first byte of a TLS client hello).
const TLS_HANDSHAKE_RECORD: u8 = 0x16;

/// Wraps a stream and prepends a byte that was already read (for protocol detection).
struct PrependByte<R> {
    first: Option<u8>,
    inner: R,
}

impl<R> AsyncRead for PrependByte<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(b) = self.first.take() {
            if buf.remaining() > 0 {
                buf.put_slice(&[b]);
                return Poll::Ready(Ok(()));
            }
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<R> AsyncWrite for PrependByte<R>
where
    R: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// SNI names we cannot serve - no matching cert for localhost.
const UNSUPPORTED_SNI: &[&str] = &["localhost", "127.0.0.1", "::1"];

/// Custom cert resolver:
/// - Case-insensitive SNI matching (DNS allows it; some clients vary)
/// - SNI "host:port" → try host part (non-standard but some clients send it)
/// - localhost / 127.0.0.1 / ::1 / no SNI → return None (no matching cert)
#[derive(Clone)]
struct CertResolverWithFallback {
    /// domain (lowercase) -> cert
    certs: HashMap<String, Arc<CertifiedKey>>,
}

impl fmt::Debug for CertResolverWithFallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CertResolverWithFallback")
            .field("domains", &self.certs.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ResolvesServerCert for CertResolverWithFallback {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?;
        let s = sni.trim();
        if s.is_empty() {
            return None;
        }
        let key = s.to_lowercase();
        if UNSUPPORTED_SNI.contains(&key.as_str()) {
            return None;
        }

        let candidates: Vec<&str> = if s.contains(':') {
            vec![s, s.split(':').next().unwrap_or(s).trim()]
        } else {
            vec![s]
        };

        for name in candidates {
            if name.is_empty() {
                continue;
            }
            if let Some(cert) = self.certs.get(&name.to_lowercase()) {
                return Some(Arc::clone(cert));
            }
        }

        None
    }
}

/// Build cert resolver from mappings with fallbacks for WebSocket/dev server connections.
fn build_cert_resolver(
    paths: &RoostPaths,
    mappings: &HashMap<String, u16>,
) -> Result<Arc<CertResolverWithFallback>> {
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

    Ok(Arc::new(CertResolverWithFallback { certs }))
}

/// HTTP redirect handler for port 80: redirect to https://host/
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

/// Start proxy server. Listens on all given ports. Port 80 (if present) redirects to HTTPS.
/// Other ports serve TLS and proxy to backends.
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
            eprintln!("Proxy listening on https://0.0.0.0:{} (TLS + plain HTTP for ws://)", port);
            tokio::spawn(async move {
                loop {
                    let (mut tcp_stream, remote_addr) = match listener.accept().await {
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
                        let mut first_byte = [0u8];
                        if tokio::io::AsyncReadExt::read_exact(&mut tcp_stream, &mut first_byte)
                            .await
                            .is_err()
                        {
                            return;
                        }
                        let first = first_byte[0];
                        let prepend = PrependByte {
                            first: Some(first),
                            inner: tcp_stream,
                        };

                        let service = |is_tls: bool| {
                            let mappings = mappings.clone();
                            let client = client.clone();
                            let remote_addr = remote_addr;
                            service_fn(move |req: Request<Incoming>| {
                                let mappings = mappings.clone();
                                let client = client.clone();
                                let remote_addr = remote_addr;
                                let is_tls = is_tls;
                                async move {
                                    match proxy_request(req, remote_addr, &mappings, &client, is_tls).await
                                    {
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
                            })
                        };

                        let result = if first == TLS_HANDSHAKE_RECORD {
                            let tls_stream = match tls_acceptor.accept(prepend).await {
                                Ok(s) => s,
                                Err(e) => {
                                    eprintln!("TLS handshake failed: {e}");
                                    return;
                                }
                            };
                            HttpBuilder::new(TokioExecutor::new())
                                .serve_connection_with_upgrades(
                                    hyper_util::rt::TokioIo::new(tls_stream),
                                    service(true),
                                )
                                .await
                        } else {
                            // Plain HTTP (e.g. ws://) - some clients send this when wss fails or as fallback
                            HttpBuilder::new(TokioExecutor::new())
                                .serve_connection_with_upgrades(
                                    hyper_util::rt::TokioIo::new(prepend),
                                    service(false),
                                )
                                .await
                        };

                        if let Err(e) = result {
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

/// Parse "host" or "host:port" into (normalized_domain, optional_port).
fn parse_host(s: &str) -> (String, Option<u16>) {
    let s = s.strip_suffix('.').unwrap_or(s).trim();
    // IPv6: [::1]:443 - port is after the closing bracket
    let (host_part, port_part) = if let Some(bracket) = s.find('[') {
        if let Some(bracket_end) = s[bracket..].find(']') {
            let end = bracket + bracket_end + 1;
            if end < s.len() && s.as_bytes().get(end) == Some(&b':') {
                (s[..end].to_string(), s[end + 1..].parse::<u16>().ok())
            } else {
                (s.to_string(), None)
            }
        } else {
            (s.to_string(), None)
        }
    } else {
        // hostname:port
        match s.rfind(':') {
            Some(colon) => {
                let (h, p) = s.split_at(colon);
                let port_str = p.trim_start_matches(':');
                match port_str.parse::<u16>() {
                    Ok(port) => (h.to_string(), Some(port)),
                    Err(_) => (s.to_string(), None),
                }
            }
            None => (s.to_string(), None),
        }
    };
    (host_part.to_lowercase(), port_part)
}

async fn proxy_request(
    mut req: Request<Incoming>,
    remote_addr: SocketAddr,
    mappings: &HashMap<String, u16>,
    client: &Client<HttpConnector, Incoming>,
    is_tls: bool,
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

    // When a specific port is in the URL (e.g. https://bjoernf.local:5173), forward to that backend.
    // Otherwise use the mapping (e.g. bjoernf.local:443 -> mapped port for main app).
    let port = if let Some(p) = explicit_port {
        if p == 443 {
            mappings.get(&domain).copied().or_else(|| {
                mappings
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&domain))
                    .map(|(_, p)| *p)
            })
        } else {
            Some(p)
        }
    } else {
        mappings.get(&domain).copied().or_else(|| {
            mappings
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&domain))
                .map(|(_, p)| *p)
        })
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

    let host = Some(domain);

    let backend = format!("http://localhost:{}", port);

    // Add X-Forwarded-* headers
    req.headers_mut()
        .insert("x-forwarded-for", remote_addr.to_string().parse().unwrap());
    req.headers_mut().insert(
        "x-forwarded-proto",
        (if is_tls { "https" } else { "http" }).parse().unwrap(),
    );
    if let Some(ref h) = host {
        req.headers_mut()
            .insert("x-forwarded-host", h.parse().unwrap());
    }

    // Preserve the original Host header (like Nginx proxy_set_header Host $host).
    // Vite and other dev servers expect it for HMR WebSocket validation.

    let uri = format!(
        "{}{}",
        backend,
        req.uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/")
    );
    *req.uri_mut() = uri.parse().unwrap();

    // Check if this is a WebSocket upgrade request.
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
        // WebSocket (or other upgrade): tunnel the connection instead of request/response.
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
                            eprintln!("WebSocket tunnel error: {e}");
                        }
                    }
                    Err(e) => eprintln!("WebSocket upgrade failed: {e}"),
                }
            });
        }

        // For 101 Switching Protocols, do NOT strip Upgrade/Connection — the client needs them.
        return Ok(Response::from_parts(parts, Full::from(Bytes::new())));
    }

    let (parts, body) = response.into_parts();
    let bytes = body.collect().await.context("read body")?.to_bytes();
    Ok(Response::from_parts(parts, Full::from(bytes)))
}
