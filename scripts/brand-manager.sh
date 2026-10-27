#!/usr/bin/env bash
# ==============================================================================
# NOTANE RETAIL SERVER - BRAND MANAGER CLI (VPS MULTI-TENANT)
# Mempermudah deploy dan manajemen banyak brand toko dalam 1 VPS
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
BRANDS_DIR="${ROOT_DIR}/brands"

mkdir -p "${BRANDS_DIR}"

show_help() {
    echo "=================================================================="
    echo "   NOTANE RETAIL SERVER - MULTI-BRAND MANAGER (VPS DOCKER)"
    echo "=================================================================="
    echo "Penggunaan: ./scripts/brand-manager.sh [COMMAND] [ARGUMEN...]"
    echo ""
    echo "Perintah:"
    echo "  create <slug> <nama_brand> <port_server> [port_db]"
    echo "         Membuat konfigurasi baru untuk brand toko."
    echo "         Contoh: ./scripts/brand-manager.sh create kopisenja \"Kopi Senja\" 18080 15432"
    echo ""
    echo "  start <slug>"
    echo "         Menjalankan server & database untuk brand tersebut."
    echo ""
    echo "  stop <slug>"
    echo "         Menghentikan server & database untuk brand tersebut."
    echo ""
    echo "  restart <slug>"
    echo "         Me-restart container untuk brand tersebut."
    echo ""
    echo "  logs <slug>"
    echo "         Melihat live logs server brand."
    echo ""
    echo "  list"
    echo "         Menampilkan daftar semua brand yang terkonfigurasi & statusnya."
    echo "=================================================================="
}

export PATH="/opt/podman/bin:/usr/local/bin:/opt/homebrew/bin:$PATH"

if command -v docker &> /dev/null; then
    CONTAINER_BIN="docker"
    COMPOSE_CMD="docker compose"
elif command -v podman &> /dev/null; then
    CONTAINER_BIN="podman"
    COMPOSE_CMD="podman compose"
else
    CONTAINER_BIN=""
    COMPOSE_CMD=""
fi

check_container_engine() {
    if [ -z "${CONTAINER_BIN}" ]; then
        echo "[!] Container engine (Docker atau Podman) tidak ditemukan di komputer/VPS ini!"
        echo "    Layanan server brand dijalankan menggunakan Container (Docker / Podman)."
        echo ""
        echo "    Panduan Instalasi:"
        echo "    - VPS Linux (Ubuntu/Debian): Jalankan 'sudo apt update && sudo apt install -y docker.io docker-compose-v2'"
        echo "    - Komputer Lokal (Mac / Windows): Pasang Docker Desktop atau Podman"
        exit 1
    fi
}

detect_host_lan_ip() {
    # 1. macOS default route interface
    if command -v route >/dev/null 2>&1 && command -v ipconfig >/dev/null 2>&1; then
        local def_if
        def_if=$(route -n get default 2>/dev/null | awk '/interface: / {print $2}')
        if [ -n "$def_if" ]; then
            local ip
            ip=$(ipconfig getifaddr "$def_if" 2>/dev/null)
            if [ -n "$ip" ]; then
                echo "$ip"
                return
            fi
        fi
    fi

    # 2. Linux hostname -I
    if command -v hostname >/dev/null 2>&1; then
        local ip
        ip=$(hostname -I 2>/dev/null | awk '{print $1}')
        if [ -n "$ip" ]; then
            echo "$ip"
            return
        fi
    fi

    echo "127.0.0.1"
}

cmd="${1:-}"

case "${cmd}" in
    create)
        slug="${2:-}"
        name="${3:-}"
        port="${4:-}"
        db_port="${5:-}"

        if [ -z "${slug}" ] || [ -z "${name}" ] || [ -z "${port}" ]; then
            echo "[!] Parameter tidak lengkap."
            echo "Format: ./scripts/brand-manager.sh create <slug> <nama_brand> <port_server> [port_db]"
            exit 1
        fi

        if [ -z "${db_port}" ]; then
            db_port=$((port - 2648)) # Default kalkulasi port database
        fi

        control_port=$((port + 1))
        env_file="${BRANDS_DIR}/${slug}.env"

        if [ -f "${env_file}" ]; then
            echo "[!] Brand '${slug}' sudah ada di ${env_file}."
            exit 1
        fi

        # Generate secure random pairing code & JWT secret
        rand_suffix=$(head /dev/urandom | LC_ALL=C tr -dc '0-9' | head -c 4)
        pairing_code="NOTANE-${rand_suffix}"
        jwt_secret=$(head /dev/urandom | LC_ALL=C tr -dc 'A-Za-z0-9' | head -c 32)
        db_pass=$(head /dev/urandom | LC_ALL=C tr -dc 'A-Za-z0-9' | head -c 16)
        detected_lan=$(detect_host_lan_ip)

        slug_db=$(echo "${slug}" | tr '-' '_')

        cat <<EOF > "${env_file}"
# Konfigurasi Brand Toko: ${name}
COMPOSE_PROJECT_NAME=notane-${slug}
BRAND_NAME=${name}
SERVER_PORT=${port}
CONTROL_PORT=${control_port}
POSTGRES_PORT=${db_port}
POSTGRES_DB=notane_${slug_db}
POSTGRES_USER=admin_${slug_db}
POSTGRES_PASSWORD=${db_pass}
PAIRING_CODE=${pairing_code}
HOST_IP=${detected_lan}
PROVIDER_SERVER_URL=${PROVIDER_SERVER_URL:-https://lisensi.notane.id}
JWT_SECRET=${jwt_secret}
RUST_LOG=notane_retail_server=info,tower_http=info
EOF

        echo "[OK] Berhasil membuat konfigurasi brand '${name}'!"
        echo "  - File Konfigurasi: brands/${slug}.env"
        echo "  - Port Web/API:     ${port}"
        echo "  - Port Kontrol:     ${control_port}"
        echo "  - Port Database:    ${db_port}"
        echo "  - Kode Pairing:     ${pairing_code}"
        echo "  - IP Host LAN:      ${detected_lan}"
        echo ""
        echo "Untuk menyalakan brand ini, jalankan:"
        echo "  ./scripts/brand-manager.sh start ${slug}"
        ;;

    start)
        check_container_engine
        slug="${2:-}"
        if [ -z "${slug}" ]; then
            echo "[!] Masukkan slug brand. Contoh: ./scripts/brand-manager.sh start kopisenja"
            exit 1
        fi
        env_file="${BRANDS_DIR}/${slug}.env"
        if [ ! -f "${env_file}" ]; then
            echo "[!] File konfigurasi ${env_file} tidak ditemukan!"
            exit 1
        fi

        detected_lan=$(detect_host_lan_ip)
        echo "[*] Memulai layanan brand '${slug}' menggunakan ${CONTAINER_BIN} (Host LAN: ${detected_lan})..."
        HOST_IP="${HOST_IP:-${detected_lan}}" ${COMPOSE_CMD} --env-file "${env_file}" -p "notane-${slug}" up -d
        echo "[OK] Layanan brand '${slug}' aktif!"
        ;;

    stop)
        check_container_engine
        slug="${2:-}"
        if [ -z "${slug}" ]; then
            echo "[!] Masukkan slug brand. Contoh: ./scripts/brand-manager.sh stop kopisenja"
            exit 1
        fi
        env_file="${BRANDS_DIR}/${slug}.env"
        if [ ! -f "${env_file}" ]; then
            echo "[!] File konfigurasi ${env_file} tidak ditemukan!"
            exit 1
        fi

        echo "[*] Menghentikan layanan brand '${slug}'..."
        ${COMPOSE_CMD} --env-file "${env_file}" -p "notane-${slug}" down
        echo "[OK] Layanan brand '${slug}' telah dihentikan."
        ;;

    restart)
        check_container_engine
        slug="${2:-}"
        if [ -z "${slug}" ]; then
            echo "[!] Masukkan slug brand. Contoh: ./scripts/brand-manager.sh restart kopisenja"
            exit 1
        fi
        env_file="${BRANDS_DIR}/${slug}.env"
        ${COMPOSE_CMD} --env-file "${env_file}" -p "notane-${slug}" restart
        echo "[OK] Layanan brand '${slug}' telah direstart."
        ;;

    logs)
        check_container_engine
        slug="${2:-}"
        if [ -z "${slug}" ]; then
            echo "[!] Masukkan slug brand. Contoh: ./scripts/brand-manager.sh logs kopisenja"
            exit 1
        fi
        env_file="${BRANDS_DIR}/${slug}.env"
        ${COMPOSE_CMD} --env-file "${env_file}" -p "notane-${slug}" logs -f server
        ;;

    list)
        echo "=================================================================="
        echo "   DAFTAR BRAND TOKO TERDAFTAR"
        echo "=================================================================="
        count=0
        for f in "${BRANDS_DIR}"/*.env; do
            [ -e "$f" ] || continue
            count=$((count + 1))
            slug=$(basename "$f" .env)
            # Source environment variables safely in subshell
            b_name=$(grep "^BRAND_NAME=" "$f" | cut -d'=' -f2- || echo "$slug")
            b_port=$(grep "^SERVER_PORT=" "$f" | cut -d'=' -f2 || echo "-")
            b_pair=$(grep "^PAIRING_CODE=" "$f" | cut -d'=' -f2 || echo "-")
            
            # Check container status
            status="STOPPED"
            if [ -n "${CONTAINER_BIN}" ]; then
                status=$(${CONTAINER_BIN} ps --filter "name=notane-${slug}-server" --format "{{.Status}}" 2>/dev/null || echo "STOPPED")
                if [ -z "$status" ]; then
                    status="STOPPED"
                fi
            fi

            echo "[$count] Brand: ${b_name} (Slug: ${slug})"
            echo "    - Port Web/API: http://IP_VPS:${b_port}"
            echo "    - Pairing Code: ${b_pair}"
            echo "    - Status:       ${status}"
            echo "------------------------------------------------------------------"
        done

        if [ "$count" -eq 0 ]; then
            echo "Belum ada brand toko yang dibuat. Buat dengan:"
            echo "  ./scripts/brand-manager.sh create <slug> <nama_brand> <port>"
        fi
        ;;

    *)
        show_help
        ;;
esac
