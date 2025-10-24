use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, error};

mod models;
use models::{CepData, SearchResult};

type DbPool = Pool<SqliteConnectionManager>;

#[derive(Clone)]
struct AppState {
    db: Arc<DbPool>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "cep_index.db".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    info!("Starting CEP Search Server...");
    info!("Database: {}", db_path);

    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager).expect("Failed to create database pool");

    let state = AppState {
        db: Arc::new(pool),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/ws/:uf/:cidade/:logradouro/json/", get(search_cep))
        .route("/search/:uf/:cidade/:logradouro", get(search_cep))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{}", port);
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "cep-search-server"
    }))
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ê' => 'e',
            'í' | 'ì' | 'î' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'û' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

async fn search_cep(
    State(state): State<AppState>,
    Path((uf, cidade, logradouro)): Path<(String, String, String)>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let uf = uf.to_uppercase();
    let cidade_norm = normalize_text(&urlencoding::decode(&cidade).unwrap_or_default());
    let logradouro_norm = normalize_text(&urlencoding::decode(&logradouro).unwrap_or_default());
    let limit = query.limit.unwrap_or(50).min(100); // Máximo 100 resultados

    info!(
        "Searching: UF={}, Cidade={}, Logradouro={}, Limit={}",
        uf, cidade_norm, logradouro_norm, limit
    );

    let db = state.db.clone();
    let results = tokio::task::spawn_blocking(move || {
        let conn = db.get().map_err(|e| {
            error!("Database connection error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT c.cep, c.logradouro, c.complemento, c.bairro, c.localidade, c.uf, c.ibge
                 FROM cep_data c
                 INNER JOIN cep_search s ON c.cep = s.cep
                 WHERE c.uf = ?1 
                   AND s.localidade_norm LIKE ?2
                   AND s.logradouro_norm LIKE ?3
                 ORDER BY c.logradouro, c.cep
                 LIMIT ?4",
            )
            .map_err(|e| {
                error!("Failed to prepare statement: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let cidade_pattern = format!("%{}%", cidade_norm);
        let logradouro_pattern = format!("%{}%", logradouro_norm);

        let rows = stmt
            .query_map(
                params![&uf, &cidade_pattern, &logradouro_pattern, limit],
                |row| {
                    Ok(CepData {
                        cep: row.get::<_, String>(0)?,
                        logradouro: row.get(1)?,
                        complemento: row.get(2).unwrap_or_default(),
                        bairro: row.get(3)?,
                        localidade: row.get(4)?,
                        uf: row.get(5)?,
                        ibge: row.get(6)?,
                    })
                },
            )
            .map_err(|e| {
                error!("Query execution error: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let mut results = Vec::new();
        for row in rows {
            match row {
                Ok(cep_data) => {
                    // Formata CEP com hífen para compatibilidade com ViaCEP
                    let mut result: SearchResult = cep_data.into();
                    if result.cep.len() == 8 {
                        result.cep = format!("{}-{}", &result.cep[..5], &result.cep[5..]);
                    }
                    results.push(result);
                }
                Err(e) => {
                    error!("Error parsing row: {}", e);
                }
            }
        }

        Ok::<Vec<SearchResult>, StatusCode>(results)
    })
    .await
    .map_err(|e| {
        error!("Task join error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })??;

    info!("Found {} results", results.len());
    Ok(Json(results))
}
