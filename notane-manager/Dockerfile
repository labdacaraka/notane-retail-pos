# ==============================================================================
# NOTANE LICENSE & UPDATE MANAGER - HIGH-PERFORMANCE DOCKERFILE
# Multi-stage build for VPS deployment (Headless API + Web Admin Dashboard)
# ==============================================================================

# ------------------------------------------------------------------------------
# STAGE 1: Build Frontend Web Manager (Vue 3 + Vite)
# ------------------------------------------------------------------------------
FROM node:20-alpine AS frontend-builder
WORKDIR /build

COPY notane-manager/package.json ./
RUN npm install

COPY notane-manager/ ./
RUN npm run build

# ------------------------------------------------------------------------------
# STAGE 2: Build Rust Provider Engine (Axum + SQLx SQLite + Tokio)
# ------------------------------------------------------------------------------
FROM rust:slim-bookworm AS backend-builder
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY notane-manager/src-tauri/Cargo.toml notane-manager/src-tauri/Cargo.lock ./
COPY notane-manager/src-tauri/src ./src
COPY notane-manager/src-tauri/build.rs ./build.rs

# Build headless daemon (no Tauri / WebKit / GTK GUI dependencies)
RUN cargo build --release --no-default-features

# ------------------------------------------------------------------------------
# STAGE 3: Minimal Production Runtime Image
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Salin binary manager dari backend-builder
COPY --from=backend-builder /build/target/release/notane-manager /usr/local/bin/notane-manager

# Salin aset frontend Web Studio dari frontend-builder
COPY --from=frontend-builder /build/dist /app/dist

# Folder persisten untuk database SQLite dan upload rilis OTA
RUN mkdir -p /data /app/releases

# Environment defaults
ENV PROVIDER_PORT=19090
ENV DATABASE_PATH=/data/notane_provider.db
ENV WEB_DIST=/app/dist
ENV RUST_LOG=notane_manager=info,tower_http=info

EXPOSE 19090

VOLUME ["/data"]

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:${PROVIDER_PORT:-19090}/health || exit 1

ENTRYPOINT ["/usr/local/bin/notane-manager", "--headless"]
