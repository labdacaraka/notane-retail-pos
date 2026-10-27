-- 062_drop_app_licenses_table.sql
-- Penghapusan tabel app_licenses dari database PostgreSQL demi alasan keamanan.
-- Lisensi kini disimpan secara eksklusif dalam brankas berkas lokal terenkripsi
-- menggunakan AES-256-GCM yang terkunci ke Hardware ID mesin (Node-Locked).

DROP TABLE IF EXISTS app_licenses CASCADE;
