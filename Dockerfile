# ============================================================
# Stage 1: Baixar base OpenCEP
# ============================================================
FROM alpine:3.18 AS downloader
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
FROM rust:1.81-alpine AS rust-builder
RUN apk add --no-cache musl-dev sqlite-dev

WORKDIR /build
COPY cep-indexer/ .

# Compilar em modo release com otimizações
RUN cargo build --release --bins

# ============================================================
# Stage 3: Indexar CEPs (criar banco SQLite)
# ============================================================
FROM alpine:3.19 AS indexer
RUN apk add --no-cache sqlite libgcc

COPY --from=rust-builder /build/target/release/indexer /usr/local/bin/indexer
COPY --from=downloader /usr/share/nginx/html/v1 /data/v1

# Criar índice SQLite
RUN JSON_DIR=/data/v1 DB_PATH=/data/cep_index.db indexer

# ============================================================
# Stage 4: Imagem runtime do servidor de busca
# ============================================================
FROM rust:1.81-alpine AS search-server
RUN apk add --no-cache libgcc

COPY --from=rust-builder /build/target/release/search-server /usr/local/bin/search-server
COPY --from=indexer /data/cep_index.db /data/cep_index.db

ENV DB_PATH=/data/cep_index.db
ENV PORT=3000
EXPOSE 3000

CMD ["search-server"]

# ============================================================
# Stage 5: Nginx (servidor principal)
# ============================================================
FROM nginx:alpine as nginx-server
RUN rm /etc/nginx/conf.d/default.conf

COPY --from=downloader /usr/share/nginx/html /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY index.html /usr/share/nginx/html/index.html

EXPOSE 80