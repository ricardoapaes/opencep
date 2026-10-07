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

The entire routing logic is in `default.conf.template` using Nginx's `try_files` and `@fallback_viacep` location:

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

- **`default.conf.template`**: All routing logic, regex patterns, proxy configuration, and fallback behavior; rendered at startup
- **`Dockerfile`**: Multi-stage build that downloads OpenCEP database (stage 1) and copies to Nginx image (stage 2)
- **`docker-compose.yml`**: Single service definition with `OPENCEP_VERSION` build arg
- **`.dockerignore`**: Excludes `v1/` directory (downloaded during build, not from local files)

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
5. **DNS resolver**: `NGINX_DNS_RESOLVER` is substituted at startup (image default `1.1.1.1`, Compose `127.0.0.11`, ECS task `169.254.169.253`); the substitution filter preserves Nginx variables
6. **Port mapping**: Container port 80 → host port 8080 (configurable in compose file)

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

- **Routing changes**: Edit `default.conf.template` regex patterns carefully - they must match ViaCEP's exact URL structure
- **Database updates**: Change `OPENCEP_VERSION` arg and rebuild with `--no-cache`
- **New endpoints**: Remember the two-tier pattern - decide if local-first or proxy-only
- **Performance**: Static file serving is intentional - avoid adding application logic outside Nginx

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
