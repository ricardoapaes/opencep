use cep_indexer::SearchEngine;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_index_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("opencep-{name}-{}-{nonce}", std::process::id()))
}

#[test]
fn cli_builds_a_versioned_search_index_from_json_files() {
    let index_path = temporary_index_path("index");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    let output = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .env("JSON_DIR", fixtures)
        .env("INDEX_PATH", &index_path)
        .env("OPENCEP_VERSION", "test-fixture")
        .output()
        .expect("indexer CLI must start");

    assert!(
        output.status.success(),
        "indexer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(index_path.join("meta.json").is_file());

    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(index_path.join("opencep-meta.json")).expect("OpenCEP metadata must exist"),
    )
    .expect("metadata must be valid JSON");

    assert_eq!(metadata["dataset_version"], "test-fixture");
    assert_eq!(metadata["document_count"], 3);
    assert_eq!(metadata["skipped_document_count"], 1);

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}

#[test]
fn failed_rebuild_preserves_the_published_immutable_index() {
    let index_path = temporary_index_path("published-index");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    let first_build = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .env("JSON_DIR", &fixtures)
        .env("INDEX_PATH", &index_path)
        .env("OPENCEP_VERSION", "published-version")
        .output()
        .expect("indexer CLI must start");
    assert!(first_build.status.success());

    let second_build = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .env("JSON_DIR", &fixtures)
        .env("INDEX_PATH", &index_path)
        .env("OPENCEP_VERSION", "replacement-version")
        .output()
        .expect("indexer CLI must start");
    assert!(!second_build.status.success());

    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(index_path.join("opencep-meta.json"))
            .expect("published metadata must remain available"),
    )
    .expect("published metadata must remain valid");
    assert_eq!(metadata["dataset_version"], "published-version");
    let preserved_index = SearchEngine::open(&index_path).expect("published index must still open");
    let preserved_results = preserved_index
        .search("SP", "Sao Paulo", "Paulista", 10)
        .expect("published index must remain searchable");
    assert_eq!(preserved_results[0].cep, "01310-100");

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}

#[test]
fn cli_rejects_invalid_data_without_publishing_a_partial_index() {
    let source_path = temporary_index_path("invalid-source");
    let index_path = temporary_index_path("invalid-index");
    std::fs::create_dir_all(&source_path).expect("temporary source must be created");
    std::fs::write(
        source_path.join("invalid.json"),
        r#"{"cep":"invalid","logradouro":"Rua A","localidade":"Cidade","uf":"SP"}"#,
    )
    .expect("invalid fixture must be written");

    let output = Command::new(env!("CARGO_BIN_EXE_indexer"))
        .env("JSON_DIR", &source_path)
        .env("INDEX_PATH", &index_path)
        .env("OPENCEP_VERSION", "invalid-fixture")
        .output()
        .expect("indexer CLI must start");

    assert!(!output.status.success());
    assert!(
        !index_path.exists(),
        "a partial index must not be published"
    );

    std::fs::remove_dir_all(source_path).expect("temporary source must be removable");
}
