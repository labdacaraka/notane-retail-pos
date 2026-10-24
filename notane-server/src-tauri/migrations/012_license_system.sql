-- Migration 012: License System & Anti-Tamper Security Audit
CREATE TABLE IF NOT EXISTS app_licenses (
    id SERIAL PRIMARY KEY,
    license_key VARCHAR(100) NOT NULL UNIQUE,
    client_name VARCHAR(255) NOT NULL,
    outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL,
    tier VARCHAR(50) NOT NULL DEFAULT 'ENTERPRISE',
    max_terminals INT NOT NULL DEFAULT 5,
    hardware_id VARCHAR(128) NOT NULL,
    license_token TEXT NOT NULL,
    signature TEXT NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    offline_grace_days INT NOT NULL DEFAULT 14,
    last_verified_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_monotonic_time BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS app_update_history (
    id SERIAL PRIMARY KEY,
    from_version VARCHAR(50) NOT NULL,
    to_version VARCHAR(50) NOT NULL,
    release_tag VARCHAR(100),
    checksum_sha256 VARCHAR(128),
    applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(50) NOT NULL DEFAULT 'SUCCESS',
    notes TEXT
);
