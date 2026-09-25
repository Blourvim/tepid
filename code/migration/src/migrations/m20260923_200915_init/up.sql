BEGIN;

CREATE TABLE IF NOT EXISTS revoked_keys (
    key_hash VARCHAR(64) PRIMARY KEY, 
    revoked_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ 
);

COMMIT;
