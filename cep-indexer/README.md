# CEP Indexer - Sistema de Busca Reversa

Sistema de indexação e busca reversa de CEPs usando Rust e SQLite, compatível com a API do ViaCEP.

## 🚀 Funcionalidades

- **Indexador**: Processa todos os arquivos JSON da base OpenCEP e cria índices SQLite otimizados
- **API de Busca**: Servidor HTTP que permite buscar CEPs por endereço (UF + Cidade + Logradouro)
- **Alta Performance**: Escrito em Rust com otimizações de release
- **Busca Normalizada**: Remove acentos e normaliza texto para melhorar resultados

## 📋 Pré-requisitos

- Rust 1.70+ (https://rustup.rs/)
- Base de dados OpenCEP (pasta `v1/` com arquivos JSON)

## 🔧 Compilação

```bash
cd cep-indexer

# Debug
cargo build

# Release (otimizado)
cargo build --release
```

## 📊 Uso - Indexador

Processa todos os JSONs e cria o banco de dados SQLite:

```bash
# Com variáveis de ambiente
JSON_DIR=/path/to/v1 DB_PATH=cep_index.db cargo run --release --bin indexer

# Ou usando defaults (busca pasta v1/ no diretório atual)
cargo run --release --bin indexer
```

**Saída esperada:**
```
INFO Starting CEP indexer...
INFO JSON directory: v1
INFO Database path: cep_index.db
INFO Creating database at: cep_index.db
INFO Processed 10000 CEPs...
INFO Processed 20000 CEPs...
INFO Indexed 1234567 CEPs successfully (123 errors)
INFO Optimizing database...
INFO Indexing completed successfully!
```

## 🌐 Uso - Servidor de Busca

Inicia o servidor HTTP na porta 3000:

```bash
# Com variáveis de ambiente
DB_PATH=cep_index.db PORT=3000 cargo run --release --bin search-server

# Ou usando defaults
cargo run --release --bin search-server
```

### Endpoints

**Health Check:**
```bash
curl http://localhost:3000/health
# {"status":"ok","service":"cep-search-server"}
```

**Busca por Endereço (compatível com ViaCEP):**
```bash
# Formato: /ws/{UF}/{Cidade}/{Logradouro}/json/
curl http://localhost:3000/ws/RS/Porto%20Alegre/Domingos/json/

# Formato alternativo: /search/{UF}/{Cidade}/{Logradouro}
curl http://localhost:3000/search/SP/São%20Paulo/Paulista

# Com limite de resultados (padrão: 50, máximo: 100)
curl "http://localhost:3000/search/RJ/Rio/Atlântica?limit=10"
```

**Resposta:**
```json
[
  {
    "cep": "01310-100",
    "logradouro": "Avenida Paulista",
    "complemento": "lado ímpar",
    "bairro": "Bela Vista",
    "localidade": "São Paulo",
    "uf": "SP",
    "ibge": "3550308"
  }
]
```

## 🗄️ Estrutura do Banco de Dados

**Tabela `cep_data`:**
- `cep` (PK): CEP sem hífen (8 dígitos)
- `logradouro`: Nome da rua/avenida/praça
- `complemento`: Informações adicionais
- `bairro`: Bairro
- `localidade`: Cidade
- `uf`: Estado (sigla)
- `ibge`: Código IBGE do município

**Tabela `cep_search` (índice normalizado):**
- `cep` (PK): Referência ao CEP
- `logradouro_norm`: Logradouro sem acentos e minúsculo
- `localidade_norm`: Cidade sem acentos e minúscula

**Índices criados:**
- `idx_uf_localidade`: Busca por estado e cidade
- `idx_logradouro`: Busca por logradouro
- `idx_uf_localidade_logradouro`: Busca composta (mais rápida)
- `idx_search_composite`: Busca normalizada

## ⚡ Performance

- **Indexação**: ~100k CEPs/minuto (depende do hardware)
- **Busca**: < 10ms para queries típicas
- **Tamanho do DB**: ~300-500MB (dependendo da base OpenCEP)

## 🐳 Integração com Docker

O Dockerfile multi-stage compila o indexador, processa os JSONs e inicia o servidor:

```dockerfile
# Stage 1: Build Rust binaries
FROM rust:1.81-alpine AS rust-builder
RUN apk add --no-cache musl-dev sqlite-dev
WORKDIR /build
COPY cep-indexer/ .
RUN cargo build --release

# Stage 2: Index CEPs
FROM alpine:3.19 AS indexer
RUN apk add --no-cache sqlite
COPY --from=rust-builder /build/target/release/indexer /usr/local/bin/
COPY v1/ /data/v1/
RUN JSON_DIR=/data/v1 DB_PATH=/data/cep_index.db indexer

# Stage 3: Runtime
FROM rust:1.81-alpine AS runtime
COPY --from=rust-builder /build/target/release/search-server /usr/local/bin/
COPY --from=indexer /data/cep_index.db /data/
ENV DB_PATH=/data/cep_index.db PORT=3000
CMD ["search-server"]
```

## 🔍 Algoritmo de Busca

1. **Normalização**: Remove acentos e converte para minúsculas
2. **Query SQL**: Usa LIKE com wildcards para busca parcial
3. **Índices**: Aproveita índices compostos para performance
4. **Limite**: Retorna no máximo 100 resultados por query

## 🛠️ Arquitetura

```
┌─────────────┐
│   v1/*.json │  (Base OpenCEP)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Indexer    │  (Rust - processa e indexa)
│  (indexer)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ SQLite DB   │  (Índices otimizados)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Server    │  (Rust - Axum HTTP)
│(search-srv) │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  HTTP API   │  (JSON responses)
└─────────────┘
```

## 🧪 Testes

```bash
# Teste unitário (quando implementado)
cargo test

# Teste de integração
cargo run --release --bin search-server &
sleep 2
curl http://localhost:3000/health
curl http://localhost:3000/ws/SP/São%20Paulo/Paulista/json/
```

## 📝 Notas Técnicas

- **Concorrência**: Usa `r2d2` para pool de conexões SQLite
- **Async**: Tokio runtime + Axum para servidor assíncrono
- **Bloqueio**: Queries SQLite rodam em `spawn_blocking` para não bloquear o runtime
- **CORS**: Habilitado por padrão para facilitar integração frontend
- **URL Encoding**: Suporta caracteres especiais em cidade/logradouro

## 📄 Licença

Mesmo esquema de licença do projeto OpenCEP principal.
