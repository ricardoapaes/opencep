use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json;
use std::fs;
use std::path::Path;
use tracing::{info, warn};
use walkdir::WalkDir;

mod models;
use models::CepData;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let json_dir = std::env::var("JSON_DIR").unwrap_or_else(|_| "v1".to_string());
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "cep_index.db".to_string());

    info!("Starting CEP indexer...");
    info!("JSON directory: {}", json_dir);
    info!("Database path: {}", db_path);

    let conn = create_database(&db_path)?;
    index_cep_files(&conn, &json_dir)?;

    info!("Indexing completed successfully!");
    Ok(())
}

fn create_database(db_path: &str) -> Result<Connection> {
    info!("Creating database at: {}", db_path);
    
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS cep_data (
            cep TEXT PRIMARY KEY,
            logradouro TEXT NOT NULL,
            complemento TEXT,
            bairro TEXT NOT NULL,
            localidade TEXT NOT NULL,
            uf TEXT NOT NULL,
            ibge TEXT NOT NULL
        )",
        [],
    )?;

    // Índices para buscas rápidas
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_uf_localidade ON cep_data(uf, localidade)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_logradouro ON cep_data(logradouro)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_uf_localidade_logradouro 
         ON cep_data(uf, localidade, logradouro)",
        [],
    )?;

    // Tabela auxiliar para busca full-text (normalizada)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS cep_search (
            cep TEXT PRIMARY KEY,
            logradouro_norm TEXT,
            localidade_norm TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_search_composite 
         ON cep_search(localidade_norm, logradouro_norm)",
        [],
    )?;

    info!("Database schema created");
    Ok(conn)
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

fn index_cep_files(conn: &Connection, json_dir: &str) -> Result<()> {
    let path = Path::new(json_dir);
    if !path.exists() {
        anyhow::bail!("JSON directory does not exist: {}", json_dir);
    }

    info!("Walking through JSON files...");
    
    let mut tx = conn.unchecked_transaction()?;
    let mut count = 0;
    let mut errors = 0;

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
    {
        match process_cep_file(&mut tx, entry.path()) {
            Ok(_) => {
                count += 1;
                if count % 10000 == 0 {
                    info!("Processed {} CEPs...", count);
                }
            }
            Err(e) => {
                errors += 1;
                warn!("Error processing {:?}: {}", entry.path(), e);
            }
        }
    }

    tx.commit()?;
    info!("Indexed {} CEPs successfully ({} errors)", count, errors);
    
    // Otimiza o banco de dados
    info!("Optimizing database...");
    conn.execute("VACUUM", [])?;
    conn.execute("ANALYZE", [])?;
    
    Ok(())
}

fn process_cep_file(tx: &mut rusqlite::Transaction, file_path: &Path) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    let cep_data: CepData = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON from: {:?}", file_path))?;

    // Remove hífen do CEP para padronização
    let cep_clean = cep_data.cep.replace("-", "");

    // Insere dados principais
    tx.execute(
        "INSERT OR REPLACE INTO cep_data (cep, logradouro, complemento, bairro, localidade, uf, ibge)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            &cep_clean,
            &cep_data.logradouro,
            &cep_data.complemento,
            &cep_data.bairro,
            &cep_data.localidade,
            &cep_data.uf,
            &cep_data.ibge,
        ],
    )?;

    // Insere dados normalizados para busca
    let logradouro_norm = normalize_text(&cep_data.logradouro);
    let localidade_norm = normalize_text(&cep_data.localidade);

    tx.execute(
        "INSERT OR REPLACE INTO cep_search (cep, logradouro_norm, localidade_norm)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![&cep_clean, &logradouro_norm, &localidade_norm],
    )?;

    Ok(())
}
