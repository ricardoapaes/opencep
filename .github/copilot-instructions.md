# OpenCEP API - Copilot Instructions

## Project Overview

This is a containerized Brazilian ZIP code (CEP) lookup API that provides 100% compatibility with ViaCEP's API. It uses a **two-tier architecture**: local static JSON files for fast lookups with automatic fallback to ViaCEP's API for missing or new CEPs.

**Tech Stack**: Nginx (Alpine), Docker multi-stage build, static JSON database

## Architecture & Data Flow

```
Request → Nginx → Try local /v1/{cep}.json → Fallback to ViaCEP proxy
```

The entire routing logic is in `nginx.conf` using Nginx's `try_files` and `@fallback_viacep` location:

1. **CEP Lookup** (`/ws/{cep}/{format}/`): Attempts local file first, proxies to ViaCEP on miss
2. **Address Search** (`/ws/{UF}/{City}/{Street}/{format}/`): Always proxies to ViaCEP (no local database)
3. **Direct Access** (`/v1/{cep}.json`): Direct local file access without fallback
4. **Health Check** (`/health`): Returns service status with timestamp (no external dependencies)

## Key Files & Responsibilities

- **`nginx.conf`**: All routing logic, regex patterns, proxy configuration, and fallback behavior
- **`Dockerfile`**: Multi-stage build that downloads OpenCEP database (stage 1) and copies to Nginx image (stage 2)
- **`docker-compose.yml`**: Single service definition with `OPENCEP_VERSION` build arg
- **`.dockerignore`**: Excludes `v1/` directory (downloaded during build, not from local files)

## Development Workflows

### Build & Run
```bash
docker compose up -d --build
```

### Test Locally
```bash
# Test health check
curl http://localhost:8080/health

# Test local cache hit
curl http://localhost:8080/ws/01001000/json/

# Test ViaCEP fallback (invalid CEP)
curl http://localhost:8080/ws/99999999/json/

# Test address search (always proxied)
curl http://localhost:8080/ws/RS/Porto%20Alegre/Domingos/json/
```

### Change Database Version
Set `OPENCEP_VERSION` in `docker-compose.yml` or environment, then rebuild with `--no-cache`.

### View Logs
```bash
docker compose logs -f --tail=200
```

## Critical Conventions

1. **No runtime data**: The `v1/` directory is created during Docker build, never committed to Git (`.gitignore`)
2. **Nginx regex patterns**: CEP patterns use named captures (`?<cep>\d{8}`) for clean proxy fallback
3. **Multi-stage build**: Stage 1 downloads/extracts database, stage 2 uses minimal nginx:alpine
4. **DNS resolver**: Hardcoded `8.8.8.8` in nginx.conf for upstream proxy resolution
5. **Port mapping**: Container port 80 → host port 8080 (configurable in compose file)

## External Dependencies

- **OpenCEP Database**: Downloaded from GitHub releases during build (`github.com/SeuAliado/OpenCEP/releases`)
- **ViaCEP API**: Fallback/proxy target at `viacep.com.br` (no authentication required)

## When Making Changes

- **Routing changes**: Edit `nginx.conf` regex patterns carefully - they must match ViaCEP's exact URL structure
- **Database updates**: Change `OPENCEP_VERSION` arg and rebuild with `--no-cache`
- **New endpoints**: Remember the two-tier pattern - decide if local-first or proxy-only
- **Performance**: Static file serving is intentional - avoid adding application logic outside Nginx

## Debugging Tips

- Check Nginx config syntax: `docker compose exec opencep-api nginx -t`
- Access container shell: `docker compose exec opencep-api sh`
- Verify database extraction: `docker compose exec opencep-api ls -lah /usr/share/nginx/html/v1/ | head`
- Test proxy connectivity: Check if container can resolve and reach `viacep.com.br`
