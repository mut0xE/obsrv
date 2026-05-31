-- Keep the gRPC stream aggregate-only.
-- forensics_history is intentionally not written by the stream processor.

ALTER TABLE wallet_analytics
    ADD COLUMN IF NOT EXISTS risk_distribution JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE program_analytics
    ADD COLUMN IF NOT EXISTS risk_distribution JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE instruction_analytics
    ADD COLUMN IF NOT EXISTS risk_distribution JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE IF NOT EXISTS alerts_history (
    id                BIGSERIAL PRIMARY KEY,
    wallet            TEXT NOT NULL,
    signature         TEXT NOT NULL,
    slot              BIGINT NOT NULL,
    risk_score        INTEGER NOT NULL,
    summary           TEXT NOT NULL,
    programs          JSONB,
    is_durable_nonce  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_alerts_wallet_created
    ON alerts_history(wallet, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_program_analytics_last_seen
    ON program_analytics(last_seen DESC);

CREATE INDEX IF NOT EXISTS idx_instruction_analytics_program_calls
    ON instruction_analytics(program_id, call_count DESC);

CREATE INDEX IF NOT EXISTS idx_instruction_daily_program_date
    ON instruction_daily(program_id, date DESC);
