# OpenCEP API - Copilot Instructions

## Project Overview

This is a containerized Brazilian ZIP code (CEP) lookup API that provides 100% compatibility with ViaCEP's API. It uses a **hybrid architecture**:

1. **CEP Lookup**: Local static JSON files for fast lookups with automatic fallback to ViaCEP
2. **Reverse Search (NEW)**: Local SQLite index (Rust) for address-to-CEP search with ViaCEP fallback

**Tech Stack**: Nginx (Alpine), Rust (Axum + SQLite), Docker multi-stage build, static JSON database

## Architecture & Data Flow

```
Request → Nginx → Try local resources → Fallback to ViaCEP

CEP Lookup:     /v1/{cep}.json (static files)
Reverse Search: Rust Server:3000 → SQLite (~500MB indexed)
```

### Routing Logic (nginx.conf)

1. **CEP Lookup** (`/ws/{cep}/{format}/`): 
   - Attempts local file first
   - Proxies to ViaCEP on miss

2. **Address Search** (`/ws/{UF}/{City}/{Street}/{format}/`):
   - 🆕 Proxies to Rust search server (SQLite index)
   - Falls back to ViaCEP if empty or error

3. **Direct Access** (`/v1/{cep}.json`): 
   - Direct local file access without fallback

4. **Health Check** (`/health`): 
   - Returns service status with timestamp

## Key Files & Responsibilities

### Infrastructure
- **`nginx.conf`**: All routing logic, regex patterns, proxy configuration
- **`Dockerfile`**: Multi-stage build (download → Rust compile → index → runtime)
- **`docker-compose.yml`**: Two services (nginx + rust search server)
- **`.dockerignore`**: Excludes `v1/` and build artifacts

### Rust Components (NEW)
- **`cep-indexer/Cargo.toml`**: Dependencies (axum, rusqlite, tokio, etc)
- **`cep-indexer/src/models.rs`**: CEP data structures
- **`cep-indexer/src/indexer.rs`**: Reads JSONs → creates SQLite with indexes
- **`cep-indexer/src/server.rs`**: HTTP server (Axum) for reverse search

### Documentation
- **`README.md`**: Main user-facing documentation
- **`docs/reverse-search.md`**: Detailed technical docs for reverse search
- **`cep-indexer/README.md`**: Rust module documentation

## Development Workflows

### Build & Run
```bash
# Habilitar BuildKit para cache otimizado (recomendado)
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

# Build completo (Rust + indexação + containers)
./build-all.sh

# Ou manual
docker compose up -d --build
```

### Test Locally
```bash
# Test health checks
curl http://localhost:8080/health  # Nginx
curl http://localhost:3000/health  # Rust server

# Test local cache hit (CEP lookup)
curl http://localhost:8080/ws/01001000/json/

# Test reverse search (NEW - local SQLite)
curl http://localhost:8080/ws/SP/São%20Paulo/Paulista/json/

# Test ViaCEP fallback (invalid CEP)
curl http://localhost:8080/ws/99999999/json/

# Run full test suite
./test-api.sh
```

### Rust Development
```bash
# Build Rust binaries locally
cd cep-indexer
cargo build --release

# Run indexer (requires v1/ directory with JSONs)
JSON_DIR=../v1 DB_PATH=cep_index.db cargo run --release --bin indexer

# Run search server
DB_PATH=cep_index.db PORT=3000 cargo run --release --bin search-server

# Test specific query
curl "http://localhost:3000/ws/RS/Porto%20Alegre/Domingos/json/"
```

### Change Database Version
Set `OPENCEP_VERSION` in `docker-compose.yml` or environment, then rebuild with `--no-cache`.

### View Logs
```bash
docker compose logs -f --tail=200           # All services
docker compose logs -f cep-search           # Rust server only
docker compose logs -f opencep-api          # Nginx only
```

## Critical Conventions

1. **No runtime data**: The `v1/` directory is created during Docker build, never committed to Git (`.gitignore`)
2. **Nginx regex patterns**: CEP patterns use named captures (`?<cep>\d{8}`) for clean proxy fallback
3. **Multi-stage build**: 
   - Stage 1: Downloads/extracts OpenCEP database
   - Stage 2: Compiles Rust binaries (indexer + search-server)
   - Stage 3: Runs indexer to create SQLite (~15-20 min for 1.5M CEPs)
   - Stage 4: Prepares Rust search server runtime
   - Stage 5: Prepares Nginx with static files
4. **BuildKit cache mounts**: Uses `--mount=type=cache` to persist downloaded ZIP between builds (~500MB saved)
5. **DNS resolver**: Hardcoded `169.254.169.253` (AWS VPC) + `8.8.8.8`/`8.8.4.4` fallback in nginx.conf
6. **Port mapping**: 
   - Nginx: container 80 → host 8080
   - Rust server: container 3000 → host 3000
7. **Rust server**: 
   - Axum async HTTP server
   - r2d2 connection pool for SQLite
   - Normalizes text (removes accents) for better search
   - Returns max 100 results per query
8. **SQLite indexes**: 
   - Composite indexes on (UF, localidade, logradouro)
   - Normalized search table for accent-insensitive queries
   - ~500MB final size with all indexes

## External Dependencies

- **OpenCEP Database**: Downloaded from GitHub releases during build (`github.com/SeuAliado/OpenCEP/releases`)
- **ViaCEP API**: Fallback/proxy target at `viacep.com.br` (no authentication required)
- **Rust Crates**: 
  - `axum`: HTTP server framework
  - `rusqlite`: SQLite interface
  - `tokio`: Async runtime
  - `r2d2`: Connection pooling
  - See `cep-indexer/Cargo.toml` for full list

## When Making Changes

### Routing Changes
- Edit `nginx.conf` regex patterns carefully - they must match ViaCEP's exact URL structure
- Remember to update both local and fallback locations
- Test with `./test-api.sh` script

### Database Updates
- Change `OPENCEP_VERSION` arg and rebuild with `--no-cache`
- Indexing takes 15-20 minutes for full database

### New Endpoints
- Decide if local-first or proxy-only
- For local-first, consider adding to Rust server
- For proxy-only, add nginx location block

### Rust Changes
- Edit files in `cep-indexer/src/`
- Rebuild Docker images (compilation happens in container)
- For local testing: `cd cep-indexer && cargo run --release --bin <binary>`

### Performance
- Static file serving is intentional - avoid adding application logic outside Nginx
- SQLite queries use prepared statements and indexes
- Connection pooling prevents database lock contention
- Consider adding Redis cache for frequently accessed queries

## Debugging Tips

- Check Nginx config syntax: `docker compose exec opencep-api nginx -t`
- Access Nginx container shell: `docker compose exec opencep-api sh`
- Access Rust container shell: `docker compose exec cep-search sh`
- Verify database extraction: `docker compose exec opencep-api ls -lah /usr/share/nginx/html/v1/ | head`
- Check SQLite database: `docker compose exec cep-search sqlite3 /data/cep_index.db "SELECT COUNT(*) FROM cep_data;"`
- Test proxy connectivity: Check if container can resolve and reach `viacep.com.br`
- Test Rust server directly: `curl http://localhost:3000/ws/SP/São%20Paulo/Paulista/json/`
- View Rust server logs: `docker compose logs -f cep-search`
- Check database indexes: `docker compose exec cep-search sqlite3 /data/cep_index.db ".indices cep_data"`

## Performance Benchmarks

### Indexing (Build Time)
- Speed: ~100k CEPs/minute (hardware dependent)
- Total time: 15-20 minutes (1.5M CEPs)
- Final DB size: ~500MB with indexes

### Runtime (Query Performance)
- CEP lookup (static file): < 1ms
- Reverse search (SQLite): < 10ms typical
- ViaCEP fallback: 100-300ms
- Concurrent requests: 1000+ req/s (local resources)
