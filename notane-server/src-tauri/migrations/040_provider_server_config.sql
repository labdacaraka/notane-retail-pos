-- 040_provider_server_config.sql
-- Keamanan Anti-Tampering: URL Lisensi ditiadakan dari database untuk mencegah manipulasi DB manager.
-- URL dikelola murni di tingkat binary Rust dan Environment Variable (PROVIDER_SERVER_URL).

DROP TABLE IF EXISTS system_provider_configs;
