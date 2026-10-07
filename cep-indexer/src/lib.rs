pub mod models;
mod normalization;

use anyhow::{bail, Context, Result};
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use models::{CepData, SearchResult};
use normalization::{normalize_street, normalize_text};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tantivy::{
    collector::TopDocs,
    doc,
    query::{BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, Query as TantivyQuery, TermQuery},
    schema::{
        Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED, STRING,
    },
    tokenizer::{LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer},
    Index, IndexReader, TantivyDocument, Term,
};
use tokio::sync::Semaphore;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};
use walkdir::WalkDir;

const SCHEMA_VERSION: u32 = 1;
const TOKENIZER_NAME: &str = "opencep";
const INDEX_METADATA_FILE: &str = "opencep-meta.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub schema_version: u32,
    pub dataset_version: String,
    pub document_count: u64,
    #[serde(default)]
    pub skipped_document_count: u64,
    pub generated_at_unix_seconds: u64,
}

#[derive(Clone, Copy)]
struct SearchFields {
    uf: Field,
    locality: Field,
    street: Field,
    record: Field,
}

fn index_schema() -> (Schema, SearchFields) {
    let indexing = TextFieldIndexing::default()
        .set_tokenizer(TOKENIZER_NAME)
        .set_index_option(IndexRecordOption::Basic);
    let searchable = TextOptions::default().set_indexing_options(indexing);
    let mut builder = Schema::builder();
    let uf = builder.add_text_field("uf", STRING);
    let locality = builder.add_text_field("locality", searchable.clone());
    let street = builder.add_text_field("street", searchable);
    let record = builder.add_text_field("record", STORED);
    let schema = builder.build();

    (
        schema,
        SearchFields {
            uf,
            locality,
            street,
            record,
        },
    )
}

fn register_tokenizer(index: &Index) {
    let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(RemoveLongFilter::limit(80))
        .filter(LowerCaser)
        .build();
    index.tokenizers().register(TOKENIZER_NAME, tokenizer);
}

pub fn build_index(
    json_dir: &Path,
    index_path: &Path,
    dataset_version: &str,
) -> Result<IndexMetadata> {
    if !json_dir.is_dir() {
        bail!("JSON directory does not exist: {}", json_dir.display());
    }
    if dataset_version.trim().is_empty() {
        bail!("dataset version cannot be empty");
    }
    if index_path.exists() {
        bail!(
            "index path already exists; publish each dataset to a new versioned path: {}",
            index_path.display()
        );
    }

    let parent = index_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create index parent {}", parent.display()))?;

    let temporary_path = sibling_path(index_path, "building");
    remove_directory_if_present(&temporary_path)?;
    fs::create_dir_all(&temporary_path)?;

    let result = build_index_in_directory(json_dir, &temporary_path, dataset_version);
    let metadata = match result {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = fs::remove_dir_all(&temporary_path);
            return Err(error);
        }
    };

    if let Err(error) = fs::rename(&temporary_path, index_path) {
        let _ = fs::remove_dir_all(&temporary_path);
        return Err(error).context("failed to publish the newly built index");
    }

    Ok(metadata)
}

fn build_index_in_directory(
    json_dir: &Path,
    index_path: &Path,
    dataset_version: &str,
) -> Result<IndexMetadata> {
    let (schema, fields) = index_schema();
    let index =
        Index::create_in_dir(index_path, schema).context("failed to create Tantivy index")?;
    register_tokenizer(&index);
    let mut writer = index.writer(100_000_000)?;
    let mut paths = WalkDir::new(json_dir)
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    paths.sort_by(|left, right| left.path().cmp(right.path()));
    let mut document_count = 0_u64;
    let mut skipped_document_count = 0_u64;

    for entry in paths.into_iter().filter(|entry| {
        entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("json")
    }) {
        let contents = fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        let cep: CepData = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", entry.path().display()))?;
        cep.validate()
            .with_context(|| format!("invalid data in {}", entry.path().display()))?;
        if cep.logradouro.trim().is_empty() {
            skipped_document_count += 1;
            continue;
        }
        let uf = cep.uf.to_uppercase();
        let locality = normalize_text(&cep.localidade);
        let street = normalize_street(&cep.logradouro);
        let result = SearchResult::from(cep);
        let record = serde_json::to_string(&result)?;

        writer.add_document(doc!(
            fields.uf => uf,
            fields.locality => locality,
            fields.street => street,
            fields.record => record,
        ))?;
        document_count += 1;
    }

    if document_count == 0 {
        bail!("no CEP JSON documents were found in {}", json_dir.display());
    }

    writer.commit()?;
    writer.wait_merging_threads()?;

    let metadata = IndexMetadata {
        schema_version: SCHEMA_VERSION,
        dataset_version: dataset_version.to_string(),
        document_count,
        skipped_document_count,
        generated_at_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before Unix epoch")?
            .as_secs(),
    };
    fs::write(
        index_path.join(INDEX_METADATA_FILE),
        serde_json::to_vec_pretty(&metadata)?,
    )?;

    let verification = SearchEngine::open(index_path)?;
    if verification.metadata.document_count != document_count {
        bail!("published document count does not match the generated metadata");
    }

    info!(
        document_count,
        skipped_document_count, "finished building Tantivy index"
    );
    Ok(metadata)
}

fn sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cep-index");
    path.with_file_name(format!(".{file_name}.{suffix}-{}", std::process::id()))
}

fn remove_directory_if_present(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove stale directory {}", path.display()))?;
    }
    Ok(())
}

#[derive(Clone)]
pub struct SearchEngine {
    reader: IndexReader,
    fields: SearchFields,
    metadata: IndexMetadata,
}

impl SearchEngine {
    pub fn open(index_path: &Path) -> Result<Self> {
        let metadata: IndexMetadata = serde_json::from_slice(
            &fs::read(index_path.join(INDEX_METADATA_FILE))
                .with_context(|| format!("missing OpenCEP metadata in {}", index_path.display()))?,
        )?;
        if metadata.schema_version != SCHEMA_VERSION {
            bail!(
                "unsupported index schema {}, expected {}",
                metadata.schema_version,
                SCHEMA_VERSION
            );
        }

        let index = Index::open_in_dir(index_path)
            .with_context(|| format!("failed to open Tantivy index at {}", index_path.display()))?;
        register_tokenizer(&index);
        let schema = index.schema();
        let fields = SearchFields {
            uf: schema.get_field("uf")?,
            locality: schema.get_field("locality")?,
            street: schema.get_field("street")?,
            record: schema.get_field("record")?,
        };
        let reader = index.reader()?;
        let indexed_documents = reader.searcher().num_docs();
        if indexed_documents != metadata.document_count {
            bail!(
                "index contains {indexed_documents} documents but metadata declares {}",
                metadata.document_count
            );
        }

        Ok(Self {
            reader,
            fields,
            metadata,
        })
    }

    pub fn metadata(&self) -> &IndexMetadata {
        &self.metadata
    }

    pub fn search(
        &self,
        uf: &str,
        locality: &str,
        street: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        validate_search(uf, locality, street)?;
        let normalized_locality = normalize_text(locality);
        let normalized_street = normalize_street(street);
        let query = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.uf, &uf.to_uppercase()),
                    IndexRecordOption::Basic,
                )),
            ),
            (
                Occur::Must,
                token_query(self.fields.locality, &normalized_locality),
            ),
            (
                Occur::Must,
                token_query(self.fields.street, &normalized_street),
            ),
        ]);
        let searcher = self.reader.searcher();
        let candidate_limit = limit.clamp(1, 100).saturating_mul(4).min(400);
        let top_documents = searcher.search(&query, &TopDocs::with_limit(candidate_limit))?;
        let mut results = Vec::with_capacity(top_documents.len());

        for (score, address) in top_documents {
            let document: TantivyDocument = searcher.doc(address)?;
            let record = document
                .get_first(self.fields.record)
                .and_then(|value| value.as_str())
                .context("indexed document is missing its stored record")?;
            results.push((score, serde_json::from_str::<SearchResult>(record)?));
        }

        results.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .partial_cmp(left_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.logradouro.cmp(&right.logradouro))
                .then_with(|| left.cep.cmp(&right.cep))
        });
        results.truncate(limit.clamp(1, 100));

        Ok(results.into_iter().map(|(_, result)| result).collect())
    }
}

fn token_query(field: Field, normalized: &str) -> Box<dyn TantivyQuery> {
    let token_queries = normalized.split_whitespace().map(|token| {
        let term = Term::from_field_text(field, token);
        let exact: Box<dyn TantivyQuery> = Box::new(BoostQuery::new(
            Box::new(TermQuery::new(term.clone(), IndexRecordOption::Basic)),
            4.0,
        ));
        let prefix: Box<dyn TantivyQuery> = Box::new(BoostQuery::new(
            Box::new(FuzzyTermQuery::new_prefix(term.clone(), 0, true)),
            2.0,
        ));
        let edit_distance = match token.chars().count() {
            0..=3 => 0,
            4..=7 => 1,
            _ => 2,
        };

        if edit_distance == 0 {
            Box::new(BooleanQuery::new(vec![
                (Occur::Should, exact),
                (Occur::Should, prefix),
            ]))
        } else {
            Box::new(BooleanQuery::new(vec![
                (Occur::Should, exact),
                (Occur::Should, prefix),
                (
                    Occur::Should,
                    Box::new(FuzzyTermQuery::new(term, edit_distance, true)),
                ),
            ])) as Box<dyn TantivyQuery>
        }
    });

    Box::new(BooleanQuery::new(
        token_queries.map(|query| (Occur::Must, query)).collect(),
    ))
}

fn validate_search(uf: &str, locality: &str, street: &str) -> Result<()> {
    if uf.len() != 2 || !uf.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("UF must contain exactly two letters");
    }
    if normalize_text(locality).chars().count() < 3 {
        bail!("locality must contain at least three characters");
    }
    if normalize_street(street).chars().count() < 3 {
        bail!("street must contain at least three characters");
    }
    Ok(())
}

#[derive(Clone)]
struct AppState {
    engine: Arc<SearchEngine>,
    search_slots: Arc<Semaphore>,
}

#[derive(Debug, Deserialize)]
struct SearchParameters {
    limit: Option<usize>,
}

pub fn create_app(engine: SearchEngine) -> Router {
    create_app_with_concurrency(engine, 16)
}

pub fn create_app_with_concurrency(engine: SearchEngine, search_concurrency: usize) -> Router {
    let state = AppState {
        engine: Arc::new(engine),
        search_slots: Arc::new(Semaphore::new(search_concurrency.max(1))),
    };

    Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/ws/:uf/:locality/:street/json", get(search_address))
        .route("/ws/:uf/:locality/:street/json/", get(search_address))
        .route("/search/:uf/:locality/:street", get(search_address))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "cep-search-server"
    }))
}

async fn readiness(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ready",
        "service": "cep-search-server",
        "dataset_version": state.engine.metadata().dataset_version,
        "schema_version": state.engine.metadata().schema_version,
        "document_count": state.engine.metadata().document_count,
        "skipped_document_count": state.engine.metadata().skipped_document_count
    }))
}

async fn search_address(
    State(state): State<AppState>,
    AxumPath((uf, locality, street)): AxumPath<(String, String, String)>,
    Query(parameters): Query<SearchParameters>,
) -> Response {
    let limit = parameters.limit.unwrap_or(50).clamp(1, 100);
    if let Err(error) = validate_search(&uf, &locality, &street) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response();
    }

    let engine = state.engine.clone();
    let permit = match state.search_slots.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(serde_json::json!({ "error": "search capacity exhausted" })),
            )
                .into_response();
        }
    };
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        engine.search(&uf, &locality, &street, limit)
    })
    .await;

    match result {
        Ok(Ok(results)) => Json(results).into_response(),
        Ok(Err(error)) => {
            error!(%error, "address search failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "address search failed" })),
            )
                .into_response()
        }
        Err(error) => {
            error!(%error, "address search task failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "address search failed" })),
            )
                .into_response()
        }
    }
}
