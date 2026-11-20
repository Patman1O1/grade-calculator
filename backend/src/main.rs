//! `grade-calculator` server binary.
//!
//! Environment:
//! - `PORT` (default 3000) and `HOST` (default 0.0.0.0): where to listen.
//! - `STATIC_DIR` (default `../frontend/dist`, or `./dist` if present): the
//!   built front-end to serve. Set it to an empty string to serve the API only.
//! - `RUST_LOG`: log filter (default `info`).

use std::net::SocketAddr;
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;

fn static_dir() -> Option<PathBuf> {
    match std::env::var("STATIC_DIR") {
        Ok(s) if s.is_empty() => None,
        Ok(s) => Some(PathBuf::from(s)),
        Err(_) => ["dist", "../frontend/dist", "frontend/dist"]
            .into_iter()
            .map(PathBuf::from)
            .find(|p| p.join("index.html").is_file()),
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let dir = static_dir();
    match &dir {
        Some(d) => tracing::info!("serving front-end from {}", d.display()),
        None => tracing::info!("no front-end build found; serving the API only"),
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, grade_calculator::app(dir.as_deref()))
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Resolves on Ctrl-C or, on Unix, SIGTERM (what container hosts send).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
