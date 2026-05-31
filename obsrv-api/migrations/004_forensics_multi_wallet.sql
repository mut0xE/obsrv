-- Allow the same transaction to appear for multiple watched wallets.
-- Previously: signature was UNIQUE (only one row per tx, stored fee_payer).
-- Now: unique on (signature, wallet) so each matched wallet gets its own row.

ALTER TABLE forensics_history DROP CONSTRAINT IF EXISTS forensics_history_signature_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_forensics_sig_wallet
    ON forensics_history(signature, wallet);
