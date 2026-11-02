#!/bin/bash
# ==============================================================================
# Notane Retail POS - macOS Complete Data & Application Uninstaller
# ==============================================================================

echo "====================================================="
echo "   NOTANE RETAIL SERVER - UNINSTALLER UNTUK MACOS    "
echo "====================================================="
echo ""
echo "Perhatian: Tindakan ini akan menghentikan seluruh proses Notane Server,"
echo "menghapus database PostgreSQL lokal, lisensi, dan cache aplikasi."
echo ""
read -p "Apakah Anda yakin ingin melanjutkan? (y/N): " confirm

if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo "Pembatalan: Proses uninstaller dihentikan."
    exit 0
fi

echo ""
echo "1. Menghentikan proses yang sedang berjalan..."
pkill -f notane-retail-server 2>/dev/null || true
pkill -f "postgres.*15432" 2>/dev/null || true
pkill -f "postgres.*notane_pos" 2>/dev/null || true
sleep 1

echo "2. Menghapus cluster database PostgreSQL dan data lisensi lokal..."
rm -rf "$HOME/Library/Application Support/notane-retail-pos"
rm -rf "$HOME/Library/Application Support/com.notane.retailserver"
rm -rf "$HOME/.notane"

echo "3. Menghapus bundle aplikasi dari /Applications jika ada..."
if [ -d "/Applications/Notane Retail Server.app" ]; then
    rm -rf "/Applications/Notane Retail Server.app"
    echo "   Aplikasi /Applications/Notane Retail Server.app berhasil dihapus."
fi

echo ""
echo "✅ Notane Retail Server dan seluruh database PostgreSQL berhasil dibersihkan!"
echo "   Sistem Anda kini dalam kondisi bersih seperti baru pertama kali instal."
echo ""

