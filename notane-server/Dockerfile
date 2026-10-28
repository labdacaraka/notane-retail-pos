# ==============================================================================
# NOTANE RETAIL SERVER - HIGH-PERFORMANCE DOCKERFILE
# Multi-stage build for VPS deployment (No GUI/Tauri overhead)
# ==============================================================================

ARG APP_VERSION=""

# ------------------------------------------------------------------------------
# STAGE 1: Build Frontend Web Manager (Vue 3 + Vite)
# ------------------------------------------------------------------------------
FROM node:20-alpine AS frontend-builder
ARG APP_VERSION
ENV VITE_APP_VERSION=${APP_VERSION}
ENV APP_VERSION=${APP_VERSION}
WORKDIR /build

COPY notane-server/package.json ./
RUN npm install

COPY notane-server/ ./
RUN npm run build

# ------------------------------------------------------------------------------
# STAGE 2: Build Rust Server Engine (Axum + SQLx + Tokio)
# ------------------------------------------------------------------------------
FROM rust:slim-bookworm AS backend-builder
ARG APP_VERSION
ENV APP_VERSION=${APP_VERSION}
WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

COPY notane-server/src-tauri/Cargo.toml notane-server/src-tauri/Cargo.lock ./
COPY notane-server/src-tauri/src ./src
COPY notane-server/src-tauri/migrations ./migrations
COPY notane-server/src-tauri/build.rs ./build.rs

# Build pure headless daemon (no Tauri / WebKit / GTK GUI dependencies)
RUN cargo build --release --no-default-features

# ------------------------------------------------------------------------------
# STAGE 3: Minimal Production Runtime Image
# ------------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
ARG APP_VERSION=""
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Salin binary server dari backend-builder
COPY --from=backend-builder /build/target/release/notane-retail-server /usr/local/bin/notane-retail-server

# Salin aset frontend Web Manager dari frontend-builder
COPY --from=frontend-builder /build/dist /app/dist

# Salin migration files untuk cadangan eksternal
COPY notane-server/src-tauri/migrations /app/migrations

# Direktori persisten untuk gambar produk dan cache
RUN mkdir -p /app/uploads /root/.notane/product_images

# Environment defaults
ENV APP_VERSION=${APP_VERSION}
ENV PORT=18080
ENV WEB_DIST=/app/dist
ENV PRODUCT_IMAGES_DIR=/app/uploads
ENV RUST_LOG=notane_retail_server=info,tower_http=info

EXPOSE 18080 18081

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:${PORT:-18080}/api/v1/system/health || exit 1

ENTRYPOINT ["/usr/local/bin/notane-retail-server"]
