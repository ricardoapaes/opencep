FROM rust:1.89-bookworm

RUN rustup component add clippy rustfmt

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        curl \
        gettext-base \
        nginx \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*
