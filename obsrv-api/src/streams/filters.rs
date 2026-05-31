// ============================================================
// streams/filters.rs — subscription filter builder
//
// Queries the database for all active watched wallets and
// programs, then builds a Yellowstone SubscribeRequest that
// filters for transactions touching any of those addresses.
//
// How monitoring works:
//   - Watched wallets/programs are added via POST /monitor/wallet
//     and POST /monitor/program endpoints.
//   - This module reads them from the DB and puts all addresses
//     into SubscribeRequestFilterTransactions.account_include.
//   - Yellowstone will then stream only transactions that have
//     at least one of those addresses in their account keys.
//   - The stream loop (yellowstone.rs) re-checks every 30s and
//     sends a new SubscribeRequest if the address set changes.
// ============================================================

use anyhow::Result;
use sqlx::PgPool;
use std::collections::HashMap;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions,
};

/// Collect all active watched wallet and program addresses from the DB.
///
/// Returns a sorted, deduplicated Vec of base58 address strings.
pub async fn collect_watched_addresses(pool: &PgPool) -> Result<Vec<String>> {
    // fetch wallets
    let wallets: Vec<String> = sqlx::query_scalar!(
        r#"SELECT wallet as "wallet!" FROM watched_wallets WHERE active = TRUE"#
    )
    .fetch_all(pool)
    .await?;

    // fetch programs
    let programs: Vec<String> = sqlx::query_scalar!(
        r#"SELECT program_id as "program_id!" FROM watched_programs WHERE active = TRUE"#
    )
    .fetch_all(pool)
    .await?;

    let mut addresses: Vec<String> = wallets;
    addresses.extend(programs);
    addresses.sort();
    addresses.dedup();

    Ok(addresses)
}

/// Build a Yellowstone SubscribeRequest that filters transactions
/// touching any of the provided addresses.
///
/// - Sets `vote: false` to skip vote transactions (noise reduction).
/// - Uses `Confirmed` commitment for balance between speed and reliability.
/// - Returns None if no addresses are being watched (caller should wait and retry).
pub fn build_subscribe_request(addresses: &[String]) -> Option<SubscribeRequest> {
    if addresses.is_empty() {
        return None;
    }

    tracing::info!(
        count = addresses.len(),
        "building subscription for {} addresses",
        addresses.len()
    );

    // transaction filter: only deliver txs that touch our watched addresses
    let tx_filter = SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: None,
        signature: None,
        account_include: addresses.to_vec(),
        account_exclude: vec![],
        account_required: vec![],
    };

    let mut transactions = HashMap::new();
    transactions.insert("obsrv".to_string(), tx_filter);

    Some(SubscribeRequest {
        // slot/block/account filters left empty — we only care about transactions
        slots: HashMap::new(),
        accounts: HashMap::new(),
        transactions,
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        // confirmed commitment
        commitment: Some(CommitmentLevel::Confirmed as i32),
        accounts_data_slice: vec![],
        ping: None,
        from_slot: None,
    })
}
