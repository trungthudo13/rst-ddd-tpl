# syntax=docker/dockerfile:1

ARG BUILDER_IMAGE=builder

FROM rust:1-slim-trixie AS toolchain

RUN sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources \
    && apt-get update \
    && apt-get install -y --no-install-recommends build-essential cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

ARG CARGO_BUILD_JOBS=1
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS} \
    CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    CARGO_PROFILE_RELEASE_STRIP=symbols

# Install the toolchain pinned by the repository before copying changing source.
COPY rust-toolchain.toml ./
RUN rustc --version && cargo --version \
    && cargo install cargo-chef --version 0.1.78 --locked

FROM toolchain AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM toolchain AS builder

COPY --from=planner /build/recipe.json ./recipe.json

# Store downloads and compiled dependencies in image layers for downstream builds.
RUN cargo chef cook --locked --release --package app --bin app --recipe-path recipe.json

# Remove the dummy build script using Cargo's current artifact layout.
RUN cargo clean --release --package app

FROM ${BUILDER_IMAGE} AS build

COPY . .
RUN cargo build --locked --release --package app --bin app \
    && install -Dm755 target/release/app /out/app

FROM gcr.io/distroless/cc-debian13:nonroot AS base

LABEL org.opencontainers.image.title="Rust with DDD template"

WORKDIR /app

# Create writable storage without depending on the Rust build stages.
COPY --chown=65532:65532 <<EOF /data/.keep
EOF

USER 65532:65532

# Single-node HiQLite defaults; override with --env or --env-file at runtime.
ENV HQL_NODE_ID=1 \
    HQL_NODES="1 127.0.0.1:18100 127.0.0.1:18200" \
    HQL_LISTEN_ADDR_RAFT=127.0.0.1 \
    HQL_LISTEN_ADDR_API=127.0.0.1 \
    HQL_DATA_DIR=/data \
    HQL_FILENAME_DB=hiqlite.db \
    HQL_SECRET_RAFT=raft-secret-change-me \
    HQL_SECRET_API=api-secret-change-me \
    HQL_LOG_SYNC=immediate_async

VOLUME ["${HQL_DATA_DIR}"]
EXPOSE 18100/tcp 18200/tcp
STOPSIGNAL SIGTERM
ENTRYPOINT ["/app/app"]

FROM base AS runtime

COPY --from=build --chown=65532:65532 /out/app /app/app
