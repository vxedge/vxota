//! Minimal HTTP /metrics responder: accepts a connection, ignores the
//! request beyond draining one read, answers with the rendered metrics.
//! Same shape as vxagent-common's — good enough for Prometheus scrapes, not
//! a general web server.

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serve `render()` output on `addr` until the task is dropped.
pub async fn serve<F>(addr: std::net::SocketAddr, render: Arc<F>) -> std::io::Result<()>
where
    F: Fn() -> String + Send + Sync + 'static,
{
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(%addr, "metrics exporter listening");
    loop {
        let (mut stream, _) = listener.accept().await?;
        let render = render.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf).await;
            let body = render();
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/plain; version=0.0.4\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.shutdown().await;
        });
    }
}
