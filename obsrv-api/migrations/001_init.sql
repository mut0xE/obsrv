-- wallets being monitored
CREATE TABLE IF NOT EXISTS watched_wallets (
    wallet           TEXT PRIMARY KEY,
    telegram_chat_id TEXT NOT NULL,
    alert_threshold  INTEGER NOT NULL DEFAULT 7,
    active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at       BIGINT NOT NULL
);

-- programs being monitored
CREATE TABLE IF NOT EXISTS watched_programs (
    program_id   TEXT PRIMARY KEY,
    name         TEXT,
    active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   BIGINT NOT NULL
);

-- every tx processed by stream or forensics
CREATE TABLE IF NOT EXISTS forensics_history (
    id               BIGSERIAL PRIMARY KEY,
    signature        TEXT NOT NULL UNIQUE,
    slot             BIGINT NOT NULL,
    wallet           TEXT NOT NULL,
    risk_score       INTEGER NOT NULL DEFAULT 0,
    execution_status TEXT NOT NULL DEFAULT 'unknown',
    failure_reason   TEXT,
    cu_consumed      BIGINT,
    fee_lamports     BIGINT NOT NULL DEFAULT 0,
    is_durable_nonce BOOLEAN NOT NULL DEFAULT FALSE,
    programs_called  JSONB,
    block_time       BIGINT,
    created_at       BIGINT NOT NULL
);

-- wallet level aggregated stats
CREATE TABLE IF NOT EXISTS wallet_analytics (
    wallet        TEXT PRIMARY KEY,
    total_txs     BIGINT NOT NULL DEFAULT 0,
    failed_txs    BIGINT NOT NULL DEFAULT 0,
    total_cu      BIGINT NOT NULL DEFAULT 0,
    avg_cu        DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_fees    BIGINT NOT NULL DEFAULT 0,
    high_risk_txs BIGINT NOT NULL DEFAULT 0,
    last_updated  BIGINT NOT NULL
);

-- wallet + program breakdown
CREATE TABLE IF NOT EXISTS wallet_program_stats (
    wallet       TEXT NOT NULL,
    program_id   TEXT NOT NULL,
    call_count   BIGINT NOT NULL DEFAULT 0,
    failed_count BIGINT NOT NULL DEFAULT 0,
    total_cu     BIGINT NOT NULL DEFAULT 0,
    avg_cu       DOUBLE PRECISION NOT NULL DEFAULT 0,
    last_called  BIGINT,
    PRIMARY KEY (wallet, program_id)
);

CREATE TABLE IF NOT EXISTS wallet_ix_stats (
    wallet           TEXT    NOT NULL,
    program_id       TEXT    NOT NULL,
    instruction_type TEXT    NOT NULL,
    call_count       BIGINT  NOT NULL DEFAULT 0,
    failed_count     BIGINT  NOT NULL DEFAULT 0,
    total_cu         BIGINT  NOT NULL DEFAULT 0,
    avg_cu           DOUBLE PRECISION NOT NULL DEFAULT 0,
    last_called      BIGINT,
    PRIMARY KEY (wallet, program_id, instruction_type)
);

-- global instruction stats across all wallets
CREATE TABLE IF NOT EXISTS instruction_analytics (
    program_id       TEXT    NOT NULL,
    instruction_type TEXT    NOT NULL,
    call_count       INTEGER NOT NULL DEFAULT 0,
    failed_count     INTEGER NOT NULL DEFAULT 0,
    total_cu         INTEGER NOT NULL DEFAULT 0,
    avg_cu           REAL    NOT NULL DEFAULT 0,
    peak_cu          INTEGER NOT NULL DEFAULT 0,
    unique_callers   INTEGER NOT NULL DEFAULT 0,
    first_called     INTEGER,
    last_called      INTEGER,
    PRIMARY KEY (program_id, instruction_type)
);

-- global program stats across all wallets
CREATE TABLE IF NOT EXISTS instruction_daily (
    program_id       TEXT NOT NULL,
    instruction_type TEXT NOT NULL,
    date             TEXT NOT NULL,
    call_count       BIGINT NOT NULL DEFAULT 0,
    failed_count     BIGINT NOT NULL DEFAULT 0,
    total_cu         BIGINT NOT NULL DEFAULT 0,
    avg_cu           DOUBLE PRECISION NOT NULL DEFAULT 0,
    peak_cu          BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (program_id, instruction_type, date)
);

CREATE TABLE IF NOT EXISTS stream_checkpoint (
    id         INTEGER PRIMARY KEY DEFAULT 1,
    last_slot  BIGINT NOT NULL DEFAULT 0,
    updated_at BIGINT NOT NULL DEFAULT 0,
    CONSTRAINT one_row CHECK (id = 1)
);

INSERT INTO stream_checkpoint (id, last_slot, updated_at)
VALUES (1, 0, 0)
ON CONFLICT DO NOTHING;

CREATE TABLE IF NOT EXISTS program_analytics (
    program_id     TEXT PRIMARY KEY,
    program_name   TEXT,
    total_calls    BIGINT NOT NULL DEFAULT 0,
    failed_calls   BIGINT NOT NULL DEFAULT 0,
    total_cu       BIGINT NOT NULL DEFAULT 0,
    avg_cu         DOUBLE PRECISION NOT NULL DEFAULT 0,
    peak_cu        BIGINT NOT NULL DEFAULT 0,
    unique_wallets BIGINT NOT NULL DEFAULT 0,
    first_seen     BIGINT,
    last_seen      BIGINT
);

-- indexes for fast lookups
CREATE INDEX IF NOT EXISTS idx_forensics_wallet
    ON forensics_history(wallet);

CREATE INDEX IF NOT EXISTS idx_forensics_created
    ON forensics_history(created_at);

CREATE INDEX IF NOT EXISTS idx_wallet_program
    ON wallet_program_stats(wallet);

CREATE INDEX IF NOT EXISTS idx_instruction_daily_date
    ON instruction_daily(date);
