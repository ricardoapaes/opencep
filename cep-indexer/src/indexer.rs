use anyhow::Result;
use cep_indexer::build_index;
use std::path::PathBuf;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let json_dir = PathBuf::from(std::env::var("JSON_DIR").unwrap_or_else(|_| "v1".to_string()));
    let index_path =
        PathBuf::from(std::env::var("INDEX_PATH").unwrap_or_else(|_| "cep_index".to_string()));
    let dataset_version =
        std::env::var("OPENCEP_VERSION").unwrap_or_else(|_| "unknown".to_string());

    info!(
        json_dir = %json_dir.display(),
        index_path = %index_path.display(),
        dataset_version,
        "building OpenCEP address index"
    );

    let metadata = build_index(&json_dir, &index_path, &dataset_version)?;
    info!(
        document_count = metadata.document_count,
        schema_version = metadata.schema_version,
        "OpenCEP address index built successfully"
    );

    Ok(())
}
