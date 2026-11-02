#!/bin/sh
set -e

# Bersihkan data cluster database PostgreSQL dan direktori aplikasi saat di-remove/purge
if [ "$1" = "purge" ] || [ "$1" = "remove" ]; then
    # Hapus data embedded_pg dan license keystore untuk root dan semua user
    rm -rf /root/.local/share/notane-retail-pos || true
    rm -rf /root/.local/share/com.notane.retailserver || true
    rm -rf /root/.notane || true

    for user_home in /home/*; do
        if [ -d "$user_home" ]; then
            rm -rf "$user_home/.local/share/notane-retail-pos" || true
            rm -rf "$user_home/.local/share/com.notane.retailserver" || true
            rm -rf "$user_home/.notane" || true
        fi
    done
fi

exit 0

