# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.89
ARG NODE_VERSION=24
ARG PNPM_VERSION=11.3.0

FROM node:${NODE_VERSION}-bookworm-slim AS web-builder
ARG PNPM_VERSION
WORKDIR /app/web
RUN corepack enable && corepack prepare pnpm@${PNPM_VERSION} --activate
COPY web/package.json web/pnpm-lock.yaml ./
RUN --mount=type=cache,id=pnpm-store,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

FROM rust:${RUST_VERSION}-bookworm AS rust-builder
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=cargo-target,target=/app/target \
    cargo build --release \
    && cp /app/target/release/dizzysync /usr/local/bin/dizzysync

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /home/dizzysync --shell /usr/sbin/nologin dizzysync \
    && mkdir -p /config /data /app/web \
    && chown -R dizzysync:dizzysync /config /data /app
COPY --from=rust-builder /usr/local/bin/dizzysync /usr/local/bin/dizzysync
COPY --from=web-builder /app/web/dist/ /app/web/
USER dizzysync
EXPOSE 8787
VOLUME ["/config", "/data"]
ENV DIZZYSYNC_OUTPUT_DIR=/data
ENTRYPOINT ["dizzysync"]
CMD ["--api-server", "--config", "/config/config.toml", "--api-bind", "0.0.0.0:8787", "--web-root", "/app/web"]
