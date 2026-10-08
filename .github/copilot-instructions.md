# OpenCEP API - Copilot Instructions

## Architecture

OpenCEP has two local lookup paths behind Nginx:

1. `/ws/{cep}/{format}/` reads static OpenCEP JSON and falls back to ViaCEP.
2. `/ws/{UF}/{city}/{street}/json/` calls the Rust/Axum search service, which
   reads an immutable Tantivy index generated from the same JSON release.

Address searches returning no matches respond `200 []` locally. ViaCEP is only
used for XML or a technical failure of the search backend. `/health` is liveness;
`/ready` verifies that the address index is loaded.

## Key files

- `default.conf.template`: Nginx routing, CORS and fallback rules.
- `Dockerfile`: dataset download, Rust build/test targets, index generation and
  runtime images.
- `docker-compose.yml`: internal `cep-search` plus public `opencep-api`.
- `cep-indexer/src/lib.rs`: index generation, validation, search and HTTP router.
- `cep-indexer/src/indexer.rs`: indexer CLI.
- `cep-indexer/src/server.rs`: HTTP process entry point.
- `cep-indexer/tests`: behavior tests at the CLI and HTTP seams.

## Development environment

Use the repository Dev Container. It pins Rust 1.89 and contains Nginx validation
tools. Do not silently execute Cargo on the host.

```bash
devcontainer exec --workspace-folder . \
  cargo test --locked --manifest-path cep-indexer/Cargo.toml \
  --test indexer_cli --test search_api

devcontainer exec --workspace-folder . \
  cargo clippy --locked --manifest-path cep-indexer/Cargo.toml \
  --all-targets -- -D warnings
```

Generate and run an index manually with:

```bash
JSON_DIR=/path/to/v1 INDEX_PATH=/tmp/cep_index OPENCEP_VERSION=2.0.1 \
  cargo run --locked --release --manifest-path cep-indexer/Cargo.toml --bin indexer

INDEX_PATH=/tmp/cep_index PORT=3000 \
  cargo run --locked --release --manifest-path cep-indexer/Cargo.toml --bin search-server
```

## Invariants

- Keep `cep-indexer/Cargo.lock` committed and use `--locked` in CI/builds.
- Never publish a partial index: all JSONs must parse and validate first.
- Open the generated index read-only at runtime and validate schema/count.
- Keep normalization identical for index and query paths.
- UF is an exact filter; city and street require at least three normalized chars.
- Do not claim latency, throughput, memory or artifact size without a benchmark
  against the full versioned dataset.
- Keep the Rust service internal to the Compose network; only Nginx exposes a
  host port.
- Compose detects the internal Docker or Podman resolver from `/etc/resolv.conf`;
  `NGINX_DNS_RESOLVER` remains available as an explicit override.
- The `v1/` dataset and generated index are build artifacts and are not committed.

## External dependencies

- OpenCEP release archive downloaded during the production image build.
- ViaCEP for direct-CEP misses, XML and technical fallback.
- Axum/Tokio for HTTP and Tantivy for embedded textual search.

See `docs/reverse-search.md` and `cep-indexer/README.md` for the contract and
index lifecycle.
