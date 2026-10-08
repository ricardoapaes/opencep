# ============================================================
# Stage 1: Baixar base OpenCEP
# ============================================================
FROM docker.io/library/alpine:3.22 AS downloader
ARG OPENCEP_VERSION=2.0.1
RUN apk update && apk add curl unzip

ARG DOWNLOAD_URL="https://github.com/SeuAliado/OpenCEP/releases/download/${OPENCEP_VERSION}/v1.zip"
ENV TARGET_DIR="/usr/share/nginx/html"

RUN --mount=type=cache,target=/cache \
    mkdir -p ${TARGET_DIR}/v1 /cache \
    && CACHE_FILE="/cache/opencep-${OPENCEP_VERSION}.zip" \
    && if [ ! -f "${CACHE_FILE}" ]; then \
        echo "Baixando base de dados OpenCEP ${OPENCEP_VERSION}..."; \
        curl -L -o "${CACHE_FILE}" "${DOWNLOAD_URL}"; \
    else \
        echo "Usando cache local: ${CACHE_FILE}"; \
    fi \
    && echo "Extraindo para ${TARGET_DIR}/v1..." \
    && unzip -o -q "${CACHE_FILE}" -d ${TARGET_DIR} \
    && echo "Download concluído: $(ls -lh ${TARGET_DIR}/v1 | wc -l) arquivos"

# ============================================================
# Stage 2: Compilar binários Rust
# ============================================================
FROM docker.io/library/rust:1.89-alpine AS rust-builder
RUN apk add --no-cache musl-dev

WORKDIR /build
COPY cep-indexer/ .

# Compilar em modo release com dependências reproduzíveis
RUN cargo build --release --locked --bins

FROM rust-builder AS rust-tests
RUN cargo test --locked --lib --tests

# ============================================================
# Stage 3: Indexar CEPs
# ============================================================
FROM docker.io/library/alpine:3.22 AS indexer-base
RUN apk add --no-cache libgcc

COPY --from=rust-builder /build/target/release/indexer /usr/local/bin/indexer

FROM indexer-base AS fixture-indexer
COPY cep-indexer/tests/fixtures /data/v1
COPY cep-indexer/tests/number_fixtures /data/v1/number_ranges
RUN JSON_DIR=/data/v1 INDEX_PATH=/data/cep_index OPENCEP_VERSION=ci-fixture indexer

FROM indexer-base AS indexer
COPY --from=downloader /usr/share/nginx/html/v1 /data/v1

# Criar índice textual versionado
ARG OPENCEP_VERSION=2.0.1
RUN JSON_DIR=/data/v1 INDEX_PATH=/data/cep_index OPENCEP_VERSION=${OPENCEP_VERSION} indexer

# ============================================================
# Stage 4: Imagem runtime do servidor de busca
# ============================================================
FROM docker.io/library/alpine:3.22 AS search-runtime
RUN apk add --no-cache libgcc

COPY --from=rust-builder /build/target/release/search-server /usr/local/bin/search-server
ENV INDEX_PATH=/data/cep_index
ENV PORT=3000
EXPOSE 3000
CMD ["search-server"]

FROM search-runtime AS search-server-test
COPY --from=fixture-indexer /data/cep_index /data/cep_index

FROM search-runtime AS search-server
COPY --from=indexer /data/cep_index /data/cep_index

# ============================================================
# Stage 5: Nginx (servidor principal)
# ============================================================
FROM docker.io/library/nginx:alpine AS nginx-base
RUN rm /etc/nginx/conf.d/default.conf

ENV NGINX_DNS_RESOLVER=1.1.1.1 \
    NGINX_ENVSUBST_FILTER=^NGINX_DNS_RESOLVER$

COPY default.conf.template /etc/nginx/templates/default.conf.template
COPY --chmod=755 16-opencep-resolver.envsh /docker-entrypoint.d/16-opencep-resolver.envsh
COPY index.html /usr/share/nginx/html/index.html
EXPOSE 80

FROM nginx-base AS nginx-server-test
COPY cep-indexer/tests/fixtures /usr/share/nginx/html/v1

FROM nginx-base AS nginx-server
COPY --from=downloader /usr/share/nginx/html /usr/share/nginx/html
