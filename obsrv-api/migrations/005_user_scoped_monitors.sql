-- Per-user wallet/program monitoring.
--
-- Before this migration the `watched_wallets` and `watched_programs` tables
-- were global — anyone calling /monitor/list would see every row. The UI now
-- authenticates each request with a Solana wallet signature (see
-- `X-User-Id` / `X-User-Auth-Msg` / `X-User-Signature` headers) and we want
-- to scope every row to the wallet that added it.

-- 1. Add a user_id column. Default '' so existing rows keep working until
--    they're re-added under a real wallet.
ALTER TABLE watched_wallets
    ADD COLUMN IF NOT EXISTS user_id TEXT NOT NULL DEFAULT '';

ALTER TABLE watched_programs
    ADD COLUMN IF NOT EXISTS user_id TEXT NOT NULL DEFAULT '';

-- 2. Replace the single-column PKs with composite (user_id, address) ones.
--    Using a DO block so we can be defensive about whatever the existing
--    primary key happens to be named.
DO $$
BEGIN
    -- watched_wallets
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'watched_wallets'::regclass AND contype = 'p'
    ) THEN
        EXECUTE (
            SELECT 'ALTER TABLE watched_wallets DROP CONSTRAINT ' || quote_ident(conname)
            FROM pg_constraint
            WHERE conrelid = 'watched_wallets'::regclass AND contype = 'p'
            LIMIT 1
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'watched_wallets'::regclass AND conname = 'watched_wallets_pkey'
    ) THEN
        ALTER TABLE watched_wallets
            ADD CONSTRAINT watched_wallets_pkey PRIMARY KEY (user_id, wallet);
    END IF;

    -- watched_programs
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'watched_programs'::regclass AND contype = 'p'
    ) THEN
        EXECUTE (
            SELECT 'ALTER TABLE watched_programs DROP CONSTRAINT ' || quote_ident(conname)
            FROM pg_constraint
            WHERE conrelid = 'watched_programs'::regclass AND contype = 'p'
            LIMIT 1
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conrelid = 'watched_programs'::regclass AND conname = 'watched_programs_pkey'
    ) THEN
        ALTER TABLE watched_programs
            ADD CONSTRAINT watched_programs_pkey PRIMARY KEY (user_id, program_id);
    END IF;
END $$;

-- 3. Indexes for the common access patterns.
CREATE INDEX IF NOT EXISTS watched_wallets_user_idx
    ON watched_wallets (user_id) WHERE active;

CREATE INDEX IF NOT EXISTS watched_programs_user_idx
    ON watched_programs (user_id) WHERE active;

-- The stream processor still needs the global "is this wallet watched
-- anywhere?" lookup, so keep a non-user index on the address columns too.
CREATE INDEX IF NOT EXISTS watched_wallets_wallet_idx
    ON watched_wallets (wallet) WHERE active;

CREATE INDEX IF NOT EXISTS watched_programs_program_idx
    ON watched_programs (program_id) WHERE active;
