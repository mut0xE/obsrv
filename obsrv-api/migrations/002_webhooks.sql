-- tracks Helius webhooks we've created so we can sync addresses
CREATE TABLE IF NOT EXISTS helius_webhooks (
    id           BIGSERIAL PRIMARY KEY,
    webhook_id   TEXT NOT NULL UNIQUE,       -- Helius webhook ID
    webhook_type TEXT NOT NULL DEFAULT 'enhanced', -- enhanced / raw
    active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   BIGINT NOT NULL,
    updated_at   BIGINT NOT NULL
);

-- stores every tx received via webhook for watched wallets/programs
CREATE TABLE IF NOT EXISTS webhook_transactions (
    id               BIGSERIAL PRIMARY KEY,
    signature        TEXT NOT NULL UNIQUE,
    slot             BIGINT NOT NULL,
    block_time       BIGINT,
    fee_payer        TEXT NOT NULL,
    fee_lamports     BIGINT NOT NULL DEFAULT 0,
    tx_type          TEXT NOT NULL DEFAULT 'UNKNOWN',
    source           TEXT,
    description      TEXT,
    success          BOOLEAN NOT NULL DEFAULT TRUE,
    error            TEXT,
    native_transfers JSONB,
    token_transfers  JSONB,
    account_data     JSONB,
    instructions     JSONB,
    events           JSONB,
    raw_payload      JSONB,
    created_at       BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_webhook_tx_fee_payer
    ON webhook_transactions(fee_payer);

CREATE INDEX IF NOT EXISTS idx_webhook_tx_slot
    ON webhook_transactions(slot);

CREATE INDEX IF NOT EXISTS idx_webhook_tx_type
    ON webhook_transactions(tx_type);

CREATE INDEX IF NOT EXISTS idx_webhook_tx_created
    ON webhook_transactions(created_at DESC);
