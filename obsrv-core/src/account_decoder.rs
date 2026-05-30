use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_account_decoder::parse_account_data::{AccountAdditionalDataV3, parse_account_data_v3};
use solana_sdk::pubkey::Pubkey;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedAccountState {
    pub address: String,
    pub owner: String,
    pub program: String,
    pub parsed: Value,
    pub space: u64,
}

pub fn decode_account_state(
    address: &Pubkey,
    owner: &Pubkey,
    data: &[u8],
) -> Option<DecodedAccountState> {
    let parsed = parse_account_data_v3(
        address,
        owner,
        data,
        Some(AccountAdditionalDataV3::default()),
    )
    .ok()?;

    Some(DecodedAccountState {
        address: address.to_string(),
        owner: owner.to_string(),
        program: parsed.program,
        parsed: parsed.parsed,
        space: parsed.space,
    })
}
