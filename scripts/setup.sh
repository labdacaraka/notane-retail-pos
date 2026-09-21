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
echo -e "\n[3/3] Memeriksa koneksi database PostgreSQL (Port 5432)..."
if nc -z 127.0.0.1 5432 2>/dev/null || (echo > /dev/tcp/127.0.0.1/5432) 2>/dev/null; then
    echo "  [OK] PostgreSQL aktif di port 5432!"
    if command -v psql &> /dev/null; then
        PGPASSWORD=postgres psql -U postgres -h 127.0.0.1 -p 5432 -c "CREATE DATABASE notane_pos;" 2>/dev/null || true
        echo "  [OK] Database 'notane_pos' telah dipastikan tersedia."
    fi
else
    echo "  [!] Port 5432 belum aktif. Memeriksa Docker..."
    if command -v docker &> /dev/null; then
        echo "  [*] Docker terdeteksi! Menjalankan database via Docker Compose..."
        docker compose up -d
        echo "  [OK] Container PostgreSQL telah dijalankan via Docker Compose di port 5432."
    else
        echo "  [!] PostgreSQL lokal belum berjalan dan Docker belum terpasang."
        echo "  [*] Silakan jalankan Docker atau instal PostgreSQL 16."
    fi
fi

echo -e "\n=================================================="
echo "  SETUP SELESAI!"
echo "  - Menjalankan Web:    npm run dev"
echo "  - Menjalankan Tauri:  npm run tauri dev"
echo "=================================================="
