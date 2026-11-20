//! Grade calculator server.
//!
//! - [`model`]: the course / category / item data model (camelCase JSON).
//! - [`engine`]: pure grading functions (drops, weighting, letters, "what do I need?").
//! - [`examples`]: example courses transcribed from the PDFs in `docs/`.
//! - [`api`]: the JSON HTTP API the React front-end calls.

pub mod api;
pub mod engine;
pub mod examples;
pub mod model;

use std::path::Path;

use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

/// The full application: the API under `/api`, plus the built front-end from
/// `static_dir` (if given) for every other path, falling back to `index.html`.
pub fn app(static_dir: Option<&Path>) -> Router {
    let mut router = Router::new().nest("/api", api::router());
    if let Some(dir) = static_dir {
        // Unknown paths get index.html (status 200) so client-side routes work.
        let spa = ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")));
        router = router.fallback_service(spa);
    }
    router
        // The front-end may be hosted on a different origin (e.g. GitHub Pages)
        // and the API holds no user data, so any origin may call it.
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}
