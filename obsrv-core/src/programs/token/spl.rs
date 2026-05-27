//! SPL Token Program Decoder
//!
//! Program ID: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
//!
//! This is Solana's original fungible token standard, analogous to ERC-20 on Ethereum.
//! All instructions are implemented in the shared `core` module since SPL Token
//! has no unique extensions.

use super::core;
use crate::types::DecodedInstruction;
use crate::types::ProgramType;

/// Entry point for SPL Token instruction decoding.
///
/// # Instruction Types (by discriminator)
/// - 3:  Transfer (basic, no mint verification)
/// - 4:  Approve (delegate spending rights)
/// - 6:  SetAuthority (change account or mint ownership)
/// - 9:  CloseAccount (recover rent SOL)
/// - 12: TransferChecked (safer transfer with mint verification)
///
/// # Future Additions (marked with //todo)
/// - 5:  Revoke (remove delegate approval)
/// - 7:  MintTo (create new tokens)
/// - 8:  Burn (destroy tokens)
/// - 10: FreezeAccount (lock account from transfers)
/// - 11: ThawAccount (unlock frozen account)
pub fn decode(
    index: usize,
    data: &[u8],
    accounts: &[u8],
    account_keys: &[String],
) -> DecodedInstruction {
    if data.is_empty() {
        return core::invalid(index, ProgramType::SplToken);
    }

    let ix_type = data[0];

    match ix_type {
        3 => core::transfer(index, data, accounts, account_keys, ProgramType::SplToken),
        4 => core::approve(index, data, accounts, account_keys, ProgramType::SplToken),
        6 => core::set_authority(index, data, accounts, account_keys, ProgramType::SplToken),
        9 => core::close_account(index, accounts, account_keys, ProgramType::SplToken),
        12 => core::transfer_checked(index, data, accounts, account_keys, ProgramType::SplToken),

        // TODO: Implement additional SPL Token instructions
        // 7 => core::mint_to(index, data, accounts, account_keys, ProgramType::SplToken),
        // 8 => core::burn(index, data, accounts, account_keys, ProgramType::SplToken),
        // 10 => core::freeze_account(index, accounts, account_keys, ProgramType::SplToken),
        // 11 => core::thaw_account(index, accounts, account_keys, ProgramType::SplToken),
        // 5  => core::revoke(index, accounts, account_keys, ProgramType::SplToken),
        _ => core::unknown_token(index, ix_type, ProgramType::SplToken, "spl_token"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_real_tx() {
        // transfer 1_000_000 raw units
        let data = [3u8, 64, 66, 15, 0, 0, 0, 0, 0];
        let accounts = [3u8, 2u8, 1u8];
        let keys = vec![
            "5CyxyYv7mkB6ZRC63gaa2xoXDMksmbN8ZXUBGgiroa4s".to_string(),
            "AArXyRJcMHG3GEBUyf42G4zUHj4wATWjTotx4LRaebkx".to_string(),
            "3ofTQQyKw2Pvur6ptG8RCJzQHtxFYSjZ3REkaefcGH4u".to_string(),
            "ETC12sNj1nS6GfxX6e3NEySAkFUxAe3HouSregBbcCnw".to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert_eq!(result.details["amount"], "1000000");
        assert_eq!(result.details["source"], keys[3]);
        assert_eq!(result.details["destination"], keys[2]);
        assert_eq!(result.details["authority"], keys[1]);
    }

    #[test]
    fn test_transfer_checked_real_tx() {
        // transfer 1_000_000 raw, 6 decimals = 1.000000 tokens
        let data = [12u8, 64, 66, 15, 0, 0, 0, 0, 0, 6];
        let accounts = [3u8, 5u8, 2u8, 1u8];
        let keys = vec![
            "2owy7CVsHTyEBeU31vdRydSk7ftVp68MfBjhb1kaPBkf".to_string(),
            "8D3CfHfEgWvDWr1k4Gnmu3P7wbyF2K2efU9Lj1MSnu75".to_string(),
            "P6vEc7LTtUPfx3sNkCy4iF2HkfChzisDUAehZebvuUo".to_string(),
            "BBpjhPUdhmshEFiZ5SfcePNM6SMEifnwZMoZspTEryK4".to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            "FRPSHRYrz3B7eZFmCGmkGxLSBhrdQBd2JkZDn3CKTarH".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert_eq!(result.details["amount_raw"], "1000000");
        assert_eq!(result.details["decimals"], "6");
        assert_eq!(result.details["amount_human"], "1.000000");
        assert_eq!(result.details["mint"], keys[5]);
        assert_eq!(result.details["source"], keys[3]);
        assert_eq!(result.details["destination"], keys[2]);
    }

    #[test]
    fn test_approve_real_tx() {
        // approve delegate to spend 1_000_000 raw units
        let data = [4u8, 64, 66, 15, 0, 0, 0, 0, 0];
        let accounts = [2u8, 4u8, 1u8];
        let keys = vec![
            "HX6XyxaGqDCNEoJaMGxDaS5hHbgHPFNebAV8HCQn1ofW".to_string(),
            "ELhjHPWorGDbQ3BLw4aqFqG5HnHGHsEiGwBamCZ4WYPg".to_string(),
            "Bn4f2Udvoe2fYBVi4qKnzAJRa8DProNKP4PtcyEDaWMT".to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            "2YzHr8ghffFdrU6PWVhRrFEW2cpTXMwcEmQCZWg6azww".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert_eq!(result.details["amount"], "1000000");
        assert_eq!(result.details["delegate"], keys[4]);
        assert_eq!(result.details["source"], keys[2]);
        assert_eq!(result.severity, crate::types::Severity::Warning);
    }

    #[test]
    fn test_close_account_real_tx() {
        // close account and send rent to destination
        let data = [9u8];
        let accounts = [2u8, 3u8, 1u8];
        let keys = vec![
            "25XPFvmguR8xvesoJVHvQDE8EcE9QQjCdixUBKPod57G".to_string(),
            "7L7BqJvQcdREpNpzKdrDWAwjE16aV81aEVjMeSRKTQ9n".to_string(),
            "13kEdAcw8Qt1hZzZWLLHPTrhNFhv1S4RXwMHYt8S5WhU".to_string(),
            "8jEhy6jrNrUtPPEToEUM9GqDNt1cjSHB986iAubsJuqm".to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert_eq!(result.severity, crate::types::Severity::Warning);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("TOKEN ACCOUNT BEING CLOSED"))
        );
        assert_eq!(result.details["account"], keys[2]);
        assert_eq!(result.details["destination"], keys[3]);
    }

    #[test]
    fn test_set_authority_real_tx() {
        // change AccountOwner authority
        let data = [
            6u8, 2, 1, 182, 78, 142, 0, 55, 108, 58, 41, 179, 213, 77, 17, 142, 233, 133, 77, 223,
            235, 199, 32, 142, 125, 104, 145, 83, 229, 88, 47, 179, 106, 244, 153,
        ];
        let accounts = [2u8, 1u8];
        let keys = vec![
            "2ysvswDmrxxTLgv51H631C23uCDe8QufFgdDA28FTNZq".to_string(),
            "Bd7cDd4Ys1nwm2oXjkbJ48Yqbk13HydJb2H7fD4unVJs".to_string(),
            "GF9gnReA281daMJfpjsLxF2UHCwDJjvBW1s8WdLH62gH".to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        ];

        let result = decode(0, &data, &accounts, &keys);

        assert_eq!(result.severity, crate::types::Severity::Critical);
        assert!(
            result
                .risk_flags
                .iter()
                .any(|f| f.contains("TOKEN AUTHORITY CHANGING"))
        );
        assert_eq!(result.details["authority_type"], "AccountOwner");
        assert_eq!(result.details["current_authority"], keys[1]);
        assert!(result.details.contains_key("new_authority"));
    }
}
