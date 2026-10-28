#!/usr/bin/env bash
# ==============================================================================
# NOTANE RETAIL POS - AUTOMATED OTA RELEASE BUNDLER & GENERATOR
# Script untuk mem-build kode terbaru, mem-bundle file ZIP rilis,
# menghitung SHA-256 Checksum, dan menyiapkan data upload ke Notane Manager.
# ==============================================================================

set -e

# Warna Terminal untuk UI CLI yang elegan
C_RESET="\033[0m"
C_BOLD="\033[1m"
C_CYAN="\033[36m"
C_GREEN="\033[32m"
C_YELLOW="\033[33m"
C_BLUE="\033[34m"
C_MAGENTA="\033[35m"
C_RED="\033[31m"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
RELEASES_DIR="${ROOT_DIR}/releases"

echo -e "${C_CYAN}${C_BOLD}"
echo "======================================================================"
echo "    📦 NOTANE RETAIL POS - AUTOMATED OTA RELEASE BUILDER             "
echo "======================================================================"
echo -e "${C_RESET}"

# Membaca parameter jika ada argumen atau masuk mode interaktif
COMPONENT="${1:-}"
VERSION="${2:-}"
RELEASE_NAME="${3:-}"
CHANGELOG="${4:-}"

# 1. Pilihan Komponen
if [ -z "$COMPONENT" ]; then
    echo -e "${C_BOLD}Pilih komponen yang ingin Anda buatkan bundle rilisnya:${C_RESET}"
    echo -e "  ${C_CYAN}1)${C_RESET} Notane Client (UI Only - Terminal Kasir & Back-Office)"
    echo -e "  ${C_MAGENTA}2)${C_RESET} Notane Server (Core Backend, Database & Host UI)"
    echo -e "  ${C_GREEN}3)${C_RESET} Keduanya (Client & Server Sekaligus)"
    read -p "Masukkan pilihan [1/2/3] (default: 1): " COMP_CHOICE
    COMP_CHOICE="${COMP_CHOICE:-1}"

    case "$COMP_CHOICE" in
        1) COMPONENT="client" ;;
        2) COMPONENT="server" ;;
        3) COMPONENT="all" ;;
        *) COMPONENT="client" ;;
    esac
fi

# 2. Input Versi
if [ -z "$VERSION" ]; then
    # Baca versi saat ini dari package.json jika ada
    CURRENT_CLIENT_VER=$(grep '"version"' "${ROOT_DIR}/notane-client/package.json" | head -1 | awk -F '"' '{print $4}')
    CURRENT_CLIENT_VER="${CURRENT_CLIENT_VER:-1.2.0}"
    read -p "Masukkan versi rilis semver [contoh: ${CURRENT_CLIENT_VER}]: " INPUT_VER
    VERSION="${INPUT_VER:-${CURRENT_CLIENT_VER}}"
fi

# 2.1 Sinkronkan versi ke seluruh manifes proyek (package.json, tauri.conf.json, Cargo.toml)
node "${ROOT_DIR}/scripts/sync-version.js" "${VERSION}"
export APP_VERSION="${VERSION}"
export VITE_APP_VERSION="${VERSION}"

# 3. Input Nama Rilis
if [ -z "$RELEASE_NAME" ]; then
    if [ "$COMPONENT" = "client" ]; then
        DEFAULT_NAME="Notane Retail Client v${VERSION} UI Update"
    elif [ "$COMPONENT" = "server" ]; then
        DEFAULT_NAME="Notane Retail Server v${VERSION} Core & UI Update"
    else
        DEFAULT_NAME="Notane Retail POS v${VERSION} Comprehensive Update"
    fi
    read -p "Nama / Judul Rilis [default: ${DEFAULT_NAME}]: " INPUT_NAME
    RELEASE_NAME="${INPUT_NAME:-${DEFAULT_NAME}}"
fi

# 4. Input Changelog
if [ -z "$CHANGELOG" ]; then
    echo -e "\n${C_BOLD}Masukkan Catatan Perubahan (Changelog):${C_RESET}"
    echo "Tips: Tuliskan poin pembaruan fitur / perbaikan bug. Tekan Enter dua kali untuk selesai:"
    CHANGELOG_LINES=()
    while IFS= read -r line; do
        [[ -z "$line" && ${#CHANGELOG_LINES[@]} -gt 0 ]] && break
        CHANGELOG_LINES+=("$line")
    done
    CHANGELOG=$(printf "%s\n" "${CHANGELOG_LINES[@]}")
    if [ -z "$CHANGELOG" ]; then
        CHANGELOG="1. Pembaruan dan perbaikan performa sistem\n2. Optimasi stabilitas antarmuka dan transaksi\n3. Sinkronisasi kompatibilitas rilis versi ${VERSION}"
    fi
fi

# 5. Khusus Client: Minimal Server Version
MIN_SERVER_VER="${5:-}"
if [ -z "$MIN_SERVER_VER" ]; then
    if [ "$COMPONENT" = "client" ] || [ "$COMPONENT" = "all" ]; then
        if [ -t 0 ]; then
            read -p "Minimal versi Notane Server yang dibutuhkan oleh Client [default: 1.1.0]: " INPUT_MIN_SRV
            MIN_SERVER_VER="${INPUT_MIN_SRV:-1.1.0}"
        else
            MIN_SERVER_VER="1.1.0"
        fi
    else
        MIN_SERVER_VER="1.1.0"
    fi
fi

# Fungsi untuk mem-build dan mem-bundle Client
build_client() {
    local VER="$1"
    local TARGET_DIR="${RELEASES_DIR}/client/v${VER}"
    mkdir -p "${TARGET_DIR}"

    echo -e "\n${C_CYAN}${C_BOLD}▶ [1/3] Membangun Frontend Notane Client UI...${C_RESET}"
    cd "${ROOT_DIR}/notane-client"
    npm run build

    echo -e "\n${C_CYAN}${C_BOLD}▶ [2/3] Mengemas Bundle ZIP Notane Client...${C_RESET}"
    local ZIP_NAME="notane-client-ui-v${VER}.zip"
    local ZIP_PATH="${TARGET_DIR}/${ZIP_NAME}"
    rm -f "${ZIP_PATH}"

    cd "${ROOT_DIR}/notane-client/dist"
    zip -q -r "${ZIP_PATH}" ./*

    echo -e "\n${C_CYAN}${C_BOLD}▶ [3/3] Menghitung SHA-256 Checksum...${C_RESET}"
    local SHA256
    if command -v shasum &> /dev/null; then
        SHA256=$(shasum -a 256 "${ZIP_PATH}" | awk '{print $1}')
    else
        SHA256=$(sha256sum "${ZIP_PATH}" | awk '{print $1}')
    fi

    local FILE_SIZE
    FILE_SIZE=$(du -h "${ZIP_PATH}" | awk '{print $1}')

    # Simpan Manifest JSON
    local MANIFEST_FILE="${TARGET_DIR}/manifest.json"
    cat <<EOF > "${MANIFEST_FILE}"
{
  "component": "client",
  "version": "${VER}",
  "release_name": "${RELEASE_NAME}",
  "changelog": "${CHANGELOG}",
  "filename": "${ZIP_NAME}",
  "filepath": "${ZIP_PATH}",
  "file_size": "${FILE_SIZE}",
  "checksum_sha256": "${SHA256}",
  "min_server_version": "${MIN_SERVER_VER}",
  "min_license_tier": "BASIC",
  "is_mandatory": false,
  "has_db_migration": false,
  "suggested_download_url": "https://update.notane.id/releases/client/v${VER}/${ZIP_NAME}",
  "created_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

    echo -e "${C_GREEN}${C_BOLD}✔ BUNDLE NOTANE CLIENT BERHASIL DIBUAT!${C_RESET}"
    echo -e "  📁 Lokasi File  : ${C_BOLD}${ZIP_PATH}${C_RESET} (${FILE_SIZE})"
    echo -e "  🔐 SHA-256 Hash : ${C_YELLOW}${C_BOLD}${SHA256}${C_RESET}"
    echo -e "  📋 Manifest     : ${TARGET_DIR}/manifest.json"
}

# Fungsi untuk mem-build dan mem-bundle Server
build_server() {
    local VER="$1"
    local TARGET_DIR="${RELEASES_DIR}/server/v${VER}"
    local STAGING_DIR="${TARGET_DIR}/staging"
    mkdir -p "${TARGET_DIR}"
    rm -rf "${STAGING_DIR}"
    mkdir -p "${STAGING_DIR}"

    echo -e "\n${C_MAGENTA}${C_BOLD}▶ [1/4] Membangun Frontend Host Notane Server UI...${C_RESET}"
    cd "${ROOT_DIR}/notane-server"
    npm run build

    echo -e "\n${C_MAGENTA}${C_BOLD}▶ [2/4] Mengompilasi Rust Binary Notane Server (Release Mode)...${C_RESET}"
    cd "${ROOT_DIR}/notane-server/src-tauri"
    cargo build --release --no-default-features

    echo -e "\n${C_MAGENTA}${C_BOLD}▶ [3/4] Mengumpulkan Aset & Mengemas Bundle ZIP Server...${C_RESET}"
    local BIN_SRC="${ROOT_DIR}/notane-server/src-tauri/target/release/notane-retail-server"
    if [ ! -f "${BIN_SRC}" ]; then
        BIN_SRC="${ROOT_DIR}/notane-server/src-tauri/target/release/notane-server"
    fi

    # Salin binary, folder web dist, dan folder migrations
    cp "${BIN_SRC}" "${STAGING_DIR}/notane-retail-server"
    chmod +x "${STAGING_DIR}/notane-retail-server"
    cp -r "${ROOT_DIR}/notane-server/dist" "${STAGING_DIR}/dist"
    cp -r "${ROOT_DIR}/notane-server/src-tauri/migrations" "${STAGING_DIR}/migrations"

    local OS_TAG="unknown"
    case "$(uname -s)" in
        Darwin*) OS_TAG="macos" ;;
        Linux*)  OS_TAG="linux" ;;
        MINGW*|MSYS*|CYGWIN*) OS_TAG="windows" ;;
    esac

    local ARCH_TAG="x64"
    case "$(uname -m)" in
        x86_64|amd64) ARCH_TAG="x64" ;;
        arm64|aarch64) ARCH_TAG="arm64" ;;
    esac

    local ZIP_NAME="notane-server-v${VER}-${OS_TAG}-${ARCH_TAG}.zip"
    local ZIP_PATH="${TARGET_DIR}/${ZIP_NAME}"
    rm -f "${ZIP_PATH}"

    cd "${STAGING_DIR}"
    zip -q -r "${ZIP_PATH}" ./*
    rm -rf "${STAGING_DIR}"

    # Buat juga salinan generic notane-server-v${VER}.zip untuk kemudahan akses
    cp "${ZIP_PATH}" "${TARGET_DIR}/notane-server-v${VER}.zip"

    echo -e "\n${C_MAGENTA}${C_BOLD}▶ [4/4] Menghitung SHA-256 Checksum...${C_RESET}"
    local SHA256
    if command -v shasum &> /dev/null; then
        SHA256=$(shasum -a 256 "${ZIP_PATH}" | awk '{print $1}')
    else
        SHA256=$(sha256sum "${ZIP_PATH}" | awk '{print $1}')
    fi

    local FILE_SIZE
    FILE_SIZE=$(du -h "${ZIP_PATH}" | awk '{print $1}')

    # Simpan Manifest JSON
    local MANIFEST_FILE="${TARGET_DIR}/manifest.json"
    cat <<EOF > "${MANIFEST_FILE}"
{
  "component": "server",
  "version": "${VER}",
  "release_name": "${RELEASE_NAME}",
  "changelog": "${CHANGELOG}",
  "filename": "${ZIP_NAME}",
  "filepath": "${ZIP_PATH}",
  "file_size": "${FILE_SIZE}",
  "checksum_sha256": "${SHA256}",
  "min_server_version": null,
  "min_license_tier": "BASIC",
  "is_mandatory": false,
  "has_db_migration": true,
  "suggested_download_url": "https://update.notane.id/releases/server/v${VER}/${ZIP_NAME}",
  "created_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

    echo -e "${C_GREEN}${C_BOLD}✔ BUNDLE NOTANE SERVER BERHASIL DIBUAT!${C_RESET}"
    echo -e "  📁 Lokasi File  : ${C_BOLD}${ZIP_PATH}${C_RESET} (${FILE_SIZE})"
    echo -e "  🔐 SHA-256 Hash : ${C_YELLOW}${C_BOLD}${SHA256}${C_RESET}"
    echo -e "  📋 Manifest     : ${TARGET_DIR}/manifest.json"
}

# Eksekusi sesuai pilihan komponen
if [ "$COMPONENT" = "client" ]; then
    build_client "${VERSION}"
elif [ "$COMPONENT" = "server" ]; then
    build_server "${VERSION}"
elif [ "$COMPONENT" = "all" ]; then
    build_server "${VERSION}"
    build_client "${VERSION}"
fi

echo -e "\n${C_GREEN}${C_BOLD}"
echo "======================================================================"
echo "    ✨ PROSES PEMBUATAN RILIS SELESAI DENGAN SUKSES!                  "
echo "======================================================================"
echo -e "${C_RESET}"
echo -e "${C_BOLD}Langkah Selanjutnya untuk Mendaftarkan ke Notane Manager:${C_RESET}"
echo -e "1. Buka Notane Manager di browser/aplikasi desktop: ${C_CYAN}http://localhost:19090${C_RESET} atau menu ${C_BOLD}'Rilis OTA'${C_RESET}"
COMP_UPPER=$(echo "$COMPONENT" | tr '[:lower:]' '[:upper:]')
echo -e "2. Klik tombol ${C_BOLD}'Rilis ${COMP_UPPER}'${C_RESET}"
echo -e "3. Salin & tempel rincian berikut ke dalam formulir rilis Notane Manager:"
echo ""

if [ "$COMPONENT" = "client" ] || [ "$COMPONENT" = "all" ]; then
    CLIENT_MANIFEST="${RELEASES_DIR}/client/v${VERSION}/manifest.json"
    if [ -f "${CLIENT_MANIFEST}" ]; then
        CLIENT_SHA=$(grep '"checksum_sha256"' "${CLIENT_MANIFEST}" | awk -F '"' '{print $4}')
        CLIENT_ZIP=$(grep '"filepath"' "${CLIENT_MANIFEST}" | awk -F '"' '{print $4}')
        echo -e "${C_CYAN}${C_BOLD}--- DATA RILIS NOTANE CLIENT ---${C_RESET}"
        echo -e " • Komponen          : ${C_BOLD}client${C_RESET}"
        echo -e " • Versi             : ${C_BOLD}${VERSION}${C_RESET}"
        echo -e " • Minimal Server    : ${C_BOLD}${MIN_SERVER_VER}${C_RESET}"
        echo -e " • Judul Rilis       : ${C_BOLD}${RELEASE_NAME}${C_RESET}"
        echo -e " • File Bundle ZIP   : ${CLIENT_ZIP}"
        echo -e " • URL Unduhan       : https://update.notane.id/releases/client/v${VERSION}/notane-client-ui-v${VERSION}.zip"
        echo -e " • Checksum SHA-256  : ${C_YELLOW}${C_BOLD}${CLIENT_SHA}${C_RESET}"
        echo ""
    fi
fi

if [ "$COMPONENT" = "server" ] || [ "$COMPONENT" = "all" ]; then
    SERVER_MANIFEST="${RELEASES_DIR}/server/v${VERSION}/manifest.json"
    if [ -f "${SERVER_MANIFEST}" ]; then
        SERVER_SHA=$(grep '"checksum_sha256"' "${SERVER_MANIFEST}" | awk -F '"' '{print $4}')
        SERVER_ZIP=$(grep '"filepath"' "${SERVER_MANIFEST}" | awk -F '"' '{print $4}')
        echo -e "${C_MAGENTA}${C_BOLD}--- DATA RILIS NOTANE SERVER ---${C_RESET}"
        echo -e " • Komponen          : ${C_BOLD}server${C_RESET}"
        echo -e " • Versi             : ${C_BOLD}${VERSION}${C_RESET}"
        echo -e " • Judul Rilis       : ${C_BOLD}${RELEASE_NAME}${C_RESET}"
        echo -e " • File Bundle ZIP   : ${SERVER_ZIP}"
        echo -e " • URL Unduhan       : https://update.notane.id/releases/server/v${VERSION}/notane-server-v${VERSION}.zip"
        echo -e " • Checksum SHA-256  : ${C_YELLOW}${C_BOLD}${SERVER_SHA}${C_RESET}"
        echo -e " • Migrasi Database  : [v] Centang 'Mengandung Migrasi Database'"
        echo ""
    fi
fi

echo -e "${C_GREEN}Tip: File ZIP di atas siap Anda upload ke storage/hosting Anda, dan data checksum di atas menjamin anti-tamper dengan tanda tangan Ed25519 Notane Manager.${C_RESET}\n"
