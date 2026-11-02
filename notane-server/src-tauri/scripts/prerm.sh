#!/bin/sh
set -e

# Hentikan proses server Notane Retail Server jika sedang aktif
pkill -f notane-retail-server || true

# Hentikan proses PostgreSQL Embedded yang berjalan di port 15432
pkill -f "postgres.*15432" || true
pkill -f "postgres.*notane_pos" || true

exit 0

