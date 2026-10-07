use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use cep_indexer::{build_index, create_app, SearchEngine};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

fn temporary_index_path(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("opencep-{name}-{}-{nonce}", std::process::id()))
}

fn fixture_index() -> (PathBuf, SearchEngine) {
    let index_path = temporary_index_path("api");
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    build_index(&fixtures, &index_path, "test-fixture").expect("fixture index must build");
    let engine = SearchEngine::open(&index_path).expect("fixture index must open");

    (index_path, engine)
}

#[tokio::test]
async fn address_search_is_accent_insensitive_and_tolerates_a_typo() {
    let (index_path, engine) = fixture_index();
    let app = create_app(engine);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/SP/Sao%20Paulo/Paulsta/json/?limit=10")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("router must answer");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body must be readable");
    let results: Vec<serde_json::Value> =
        serde_json::from_slice(&body).expect("response must be a JSON array");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["cep"], "01310-100");
    assert_eq!(results[0]["logradouro"], "Avenida Paulista");
    assert_eq!(
        results[0],
        serde_json::json!({
            "cep": "01310-100",
            "logradouro": "Avenida Paulista",
            "complemento": "lado par",
            "unidade": "",
            "bairro": "Bela Vista",
            "localidade": "São Paulo",
            "uf": "SP",
            "estado": "São Paulo",
            "regiao": "Sudeste",
            "ibge": "3550308",
            "gia": "1004",
            "ddd": "11",
            "siafi": "7107"
        })
    );

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}

#[tokio::test]
async fn address_search_matches_a_street_term_prefix() {
    let (index_path, engine) = fixture_index();
    let app = create_app(engine);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/SP/Sao%20Paulo/Paul/json")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("router must answer");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body must be readable");
    let results: Vec<serde_json::Value> =
        serde_json::from_slice(&body).expect("response must be a JSON array");
    assert_eq!(results[0]["cep"], "01310-100");

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}

#[tokio::test]
async fn readiness_reports_the_loaded_dataset() {
    let (index_path, engine) = fixture_index();
    let app = create_app(engine);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("router must answer");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body must be readable");
    let readiness: serde_json::Value =
        serde_json::from_slice(&body).expect("response must be valid JSON");

    assert_eq!(readiness["status"], "ready");
    assert_eq!(readiness["dataset_version"], "test-fixture");
    assert_eq!(readiness["document_count"], 3);
    assert_eq!(readiness["skipped_document_count"], 1);

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}

#[tokio::test]
async fn invalid_search_parameters_return_a_client_error() {
    let (index_path, engine) = fixture_index();
    let app = create_app(engine);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/S/Sao%20Paulo/Pa/json")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("router must answer");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}

#[tokio::test]
async fn search_without_matches_returns_an_empty_array_without_external_fallback() {
    let (index_path, engine) = fixture_index();
    let app = create_app(engine);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ws/SP/Sao%20Paulo/Inexistente/json")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("router must answer");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("response body must be readable");
    assert_eq!(body.as_ref(), b"[]");

    std::fs::remove_dir_all(index_path).expect("temporary index must be removable");
}
