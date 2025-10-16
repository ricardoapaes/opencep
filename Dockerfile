FROM alpine:3.18
ARG OPENCEP_VERSION=2.0.1
RUN apk update && apk add curl unzip
ENV DOWNLOAD_URL="https://github.com/SeuAliado/OpenCEP/releases/download/${OPENCEP_VERSION}/v1.zip"
ENV TARGET_DIR="/usr/share/nginx/html"
ENV TEMP_ZIP="/tmp/opencep-data.zip"

RUN mkdir -p ${TARGET_DIR}/v1 \
    && echo "Baixando base de dados..." \
    && curl -L -o ${TEMP_ZIP} ${DOWNLOAD_URL}

RUN echo "Extraindo para ${TARGET_DIR}/v1..." \
 && unzip -o -q ${TEMP_ZIP} -d ${TARGET_DIR}

RUN echo "Limpando temporários..." \
 && rm -f ${TEMP_ZIP}

FROM nginx:alpine
RUN rm /etc/nginx/conf.d/default.conf
COPY --from=0 /usr/share/nginx/html /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80