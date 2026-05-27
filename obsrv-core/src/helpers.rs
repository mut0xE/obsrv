// Helper Functions
/// Reads a u64 value from a byte slice at the given offset (little-endian).
///
/// Solana stores all integer values as little-endian. This helper safely reads
/// 8 bytes starting at `offset` and converts them to a u64.
///
/// Returns 0 if there aren't enough bytes — callers should bounds-check
/// before calling if they need to distinguish "0 lamports" from "truncated data".
///
/// # Example
/// ```text
/// data = [2, 0, 0, 0,  64, 66, 15, 0, 0, 0, 0, 0]
///                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
///                       offset=4, reads these 8 bytes
///                       = 1_000_000 (0.001 SOL)
/// ```
pub fn read_u64(data: &[u8], offset: usize) -> u64 {
    if data.len() >= offset + 8 {
        u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0; 8]))
    } else {
        0
    }
}

/// Resolves an account address from the accounts indirection array.
///
/// # How Account Indirection Works
///
/// Each instruction doesn't store full account addresses — it stores indices.
/// The `accounts` array contains positions into the transaction's `account_keys`.
///
/// ```text
/// account_keys = ["ABC...", "DEF...", "GHI...", "111..."]
/// accounts     = [0, 2]
///
/// get_account(accounts, 0, account_keys)  → "ABC..." (accounts[0] = 0 → account_keys[0])
/// get_account(accounts, 1, account_keys)  → "GHI..." (accounts[1] = 2 → account_keys[2])
/// ```
///
/// Returns "unknown" if the position is out of bounds
pub fn get_account(accounts: &[u8], position: usize, account_keys: &[String]) -> String {
    accounts
        .get(position)
        .and_then(|&idx| account_keys.get(idx as usize))
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}
