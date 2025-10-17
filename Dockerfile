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

FROM nginx:alpine
RUN rm /etc/nginx/conf.d/default.conf
COPY --from=downloader /usr/share/nginx/html /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80