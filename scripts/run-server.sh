#!/usr/bin/env bash
# ==============================================================================
# NOTANE RETAIL SERVER - RUN DIRECT NATIVE (FAST DEV / DEBUGGING)
# Menjalankan server langsung dari Cargo tanpa container Podman
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BRANDS_DIR="${ROOT_DIR}/brands"

BRAND_SLUG="${1:-riang-market}"
ENV_FILE="${BRANDS_DIR}/${BRAND_SLUG}.env"

if [ -f "${ENV_FILE}" ]; then
    echo ">> Memuat konfigurasi brand dari: ${ENV_FILE}"
    while IFS='=' read -r key val || [ -n "$key" ]; do
        # Abaikan komentar dan baris kosong
        [[ "$key" =~ ^[[:space:]]*# ]] && continue
        [[ -z "$key" ]] && continue
        # Hilangkan spasi dan tanda petik
        key=$(echo "$key" | tr -d '[:space:]')
        val=$(echo "$val" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/^"//' -e 's/"$//' -e "s/^'//" -e "s/'$//")
        export "$key"="$val"
    done < "${ENV_FILE}"
else
    echo ">> File konfigurasi brand '${ENV_FILE}' tidak ditemukan, menggunakan nilai bawaan..."
fi

# Pastikan port mapping sesuai
export PORT="${SERVER_PORT:-15438}"
export CONTROL_PORT="${CONTROL_PORT:-15439}"
export PG_PORT="${POSTGRES_PORT:-12790}"
export PG_DB="${POSTGRES_DB:-notane_riang-market}"
export PG_USER="${POSTGRES_USER:-admin_riang-market}"
export PG_PASS="${POSTGRES_PASSWORD:-reD9MQMfPzU6uDl3}"

export DATABASE_URL="${DATABASE_URL:-postgres://${PG_USER}:${PG_PASS}@localhost:${PG_PORT}/${PG_DB}}"
export PAIRING_CODE="${PAIRING_CODE:-NOTANE-6749}"
export BRAND_NAME="${BRAND_NAME:-Riang Market}"
export JWT_SECRET="${JWT_SECRET:-GdGkbRPPoK3FineC2usoo0DyUdktXTr7}"
export RUST_LOG="${RUST_LOG:-notane_retail_server=info,tower_http=info}"
export WEB_DIST="${WEB_DIST:-${ROOT_DIR}/notane-server/dist}"
export PRODUCT_IMAGES_DIR="${PRODUCT_IMAGES_DIR:-${ROOT_DIR}/uploads}"

mkdir -p "${PRODUCT_IMAGES_DIR}"

echo "====================================================="
echo "   NOTANE RETAIL SERVER (NATIVE CARGO RUNNER)        "
echo "====================================================="
echo " Brand Toko    : ${BRAND_NAME} (${BRAND_SLUG})"
echo " API Port      : http://127.0.0.1:${PORT}"
echo " Control Port  : http://127.0.0.1:${CONTROL_PORT}"
echo " Database URL  : ${DATABASE_URL}"
echo " Pairing Code  : ${PAIRING_CODE}"
echo " Logging       : ${RUST_LOG}"
echo "====================================================="
echo ">> Menjalankan cargo run (headless)... Tekan Ctrl+C untuk berhenti."
echo ""

cd "${ROOT_DIR}/notane-server/src-tauri"
exec cargo run --no-default-features -- --headless
