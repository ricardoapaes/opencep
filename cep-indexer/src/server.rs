use anyhow::{Context, Result};
use cep_indexer::{create_app_with_concurrency, SearchEngine};
use std::path::PathBuf;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let index_path =
        PathBuf::from(std::env::var("INDEX_PATH").unwrap_or_else(|_| "cep_index".to_string()));
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let search_concurrency = std::env::var("SEARCH_CONCURRENCY")
        .unwrap_or_else(|_| "16".to_string())
        .parse::<usize>()
        .context("SEARCH_CONCURRENCY must be a positive integer")?;
    if search_concurrency == 0 {
        anyhow::bail!("SEARCH_CONCURRENCY must be greater than zero");
    }
    let engine = SearchEngine::open(&index_path)
        .with_context(|| format!("failed to open index at {}", index_path.display()))?;
    let app = create_app_with_concurrency(engine, search_concurrency);
    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .with_context(|| format!("failed to bind server to {address}"))?;

    info!(address, index_path = %index_path.display(), "CEP search server ready");
    axum::serve(listener, app)
        .await
        .context("search server failed")
}
