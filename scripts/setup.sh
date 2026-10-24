#!/usr/bin/env bash
set -e

echo "=================================================="
echo "   NOTANE RETAIL POS - AUTOMATED SETUP (LINUX/MAC) "
echo "=================================================="

# 1. Environment Config
echo -e "\n[1/3] Memeriksa konfigurasi .env..."
if [ ! -f ".env" ]; then
    if [ -f ".env.example" ]; then
        cp .env.example .env
        echo "  [OK] Berhasil membuat .env dari .env.example"
    else
        echo "  [!] File .env.example tidak ditemukan!"
    fi
else
    echo "  [OK] File .env sudah tersedia."
fi

# 2. Frontend Dependencies
echo -e "\n[2/3] Memeriksa dependensi NPM..."
if [ ! -d "node_modules" ]; then
    echo "  [*] Menjalankan npm install..."
    npm install
    echo "  [OK] Dependensi NPM berhasil dipasang."
else
    echo "  [OK] Folder node_modules sudah tersedia."
fi

# 3. Database Check
echo -e "\n[3/3] Memeriksa koneksi database PostgreSQL terisolasi (Port 15432)..."
if nc -z 127.0.0.1 15432 2>/dev/null || (echo > /dev/tcp/127.0.0.1/15432) 2>/dev/null; then
    echo "  [OK] PostgreSQL terisolasi aktif di port 15432!"
    if command -v psql &> /dev/null; then
        PGPASSWORD=postgres psql -U postgres -h 127.0.0.1 -p 15432 -c "CREATE DATABASE notane_pos;" 2>/dev/null || true
        echo "  [OK] Database 'notane_pos' telah dipastikan tersedia."
    fi
else
    echo "  [!] Port 15432 belum aktif. Memeriksa Docker/Podman..."
    if command -v docker &> /dev/null; then
        echo "  [*] Docker terdeteksi! Menjalankan database via Docker Compose..."
        docker compose up -d
        echo "  [OK] Container PostgreSQL telah dijalankan via Docker Compose di port terisolasi 15432."
    elif command -v podman &> /dev/null; then
        echo "  [*] Podman terdeteksi! Menjalankan container di port terisolasi 15432..."
        podman run -d --name notane-postgres -p 15432:5432 -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=notane_pos docker.io/library/postgres:16-alpine || true
    else
        echo "  [!] Database container belum berjalan. Aplikasi akan otomatis menjalankan Embedded PostgreSQL."
    fi
fi

echo -e "\n=================================================="
echo "  SETUP SELESAI!"
echo "  - Menjalankan Client POS:    npm run client:app"
echo "  - Menjalankan Server Host:   npm run server:app"
echo "  - Menjalankan Semua Dev:     npm run dev:all"
echo "=================================================="
