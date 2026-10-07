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

FROM nginx:alpine as cep
RUN rm /etc/nginx/conf.d/default.conf
COPY --from=downloader /usr/share/nginx/html /usr/share/nginx/html

FROM cep
ENV NGINX_DNS_RESOLVER=1.1.1.1 \
    NGINX_ENVSUBST_FILTER=^NGINX_DNS_RESOLVER$
COPY default.conf.template /etc/nginx/templates/default.conf.template
COPY index.html /usr/share/nginx/html/index.html
EXPOSE 80
