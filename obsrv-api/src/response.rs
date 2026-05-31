use obsrv_core::types::*;
use solana_sdk::native_token::LAMPORTS_PER_SOL;

pub fn severity_str(s: &Severity) -> String {
    match s {
        Severity::None => "none",
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Critical => "critical",
    }
    .to_string()
}

pub fn program_str(p: &ProgramType) -> String {
    match p {
        ProgramType::System => "System".to_string(),
        ProgramType::SplToken => "SPL Token".to_string(),
        ProgramType::Token2022 => "Token-2022".to_string(),
        ProgramType::ComputeBudget => "Compute Budget".to_string(),
        ProgramType::Unknown(s) => s.clone(),
    }
}

pub fn program_id_str(p: &ProgramType) -> String {
    match p {
        ProgramType::System => "11111111111111111111111111111111".to_string(),
        ProgramType::SplToken => "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
        ProgramType::Token2022 => "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string(),
        ProgramType::ComputeBudget => "ComputeBudget111111111111111111111111111111".to_string(),
        ProgramType::Unknown(s) => s.clone(),
    }
}

pub fn ix_type_str(t: &InstructionType) -> String {
    match t {
        InstructionType::NonceAdvance => "nonce_advance",
        InstructionType::NonceInitialize => "nonce_initialize",
        InstructionType::NonceWithdraw => "nonce_withdraw",
        InstructionType::NonceAuthorize => "nonce_authorize",
        InstructionType::CreateAccount => "create_account",
        InstructionType::Transfer => "transfer",
        InstructionType::TokenTransfer => "token_transfer",
        InstructionType::TokenTransferChecked => "token_transfer_checked",
        InstructionType::TokenCloseAccount => "token_close_account",
        InstructionType::TokenSetAuthority => "token_set_authority",
        InstructionType::TokenApprove => "token_approve",
        InstructionType::SetComputeUnitLimit => "set_compute_unit_limit",
        InstructionType::SetComputeUnitPrice => "set_compute_unit_price",
        InstructionType::Unknown(s) => return format!("unknown_{}", s),
    }
    .to_string()
}

pub fn risk_level_str(s: &Severity) -> String {
    match s {
        Severity::None => "low",
        Severity::Info => "medium",
        Severity::Warning => "high",
        Severity::Critical => "critical",
    }
    .to_string()
}

pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

pub fn build_meta(report: &TransactionReport) -> TxMeta {
    TxMeta {
        fee_payer: report.fee_payer.clone(),
        is_durable_nonce: report.is_durable_nonce,
        nonce: report.is_durable_nonce.then(|| NonceDetail {
            account: report.nonce_account.clone().unwrap_or_default(),
            authority: report.nonce_authority.clone().unwrap_or_default(),
        }),
    }
}

pub fn build_analysis(report: &TransactionReport) -> TxAnalysis {
    TxAnalysis {
        risk_score: report.risk_score,
        risk_level: risk_level_str(&report.risk_level),
        recommendation: report.recommendation.clone(),
        summary: report.summary.clone(),
        flags: report.risk_flags.clone(),
    }
}

pub fn build_instructions(report: &TransactionReport) -> Vec<TxInstruction> {
    report
        .instructions
        .iter()
        .map(|ix| {
            let amount_raw = ix
                .details
                .get("amount_raw")
                .or_else(|| ix.details.get("amount"))
                .cloned();

            let token = amount_raw.map(|amount_raw| {
                let decimals = ix
                    .details
                    .get("decimals")
                    .and_then(|d| d.parse::<u8>().ok());

                let amount_ui = decimals.map(|d| raw_to_ui_string(&amount_raw, d));

                InstructionToken {
                    mint: ix.details.get("mint").cloned(),
                    amount_raw,
                    amount_ui,
                    decimals,
                }
            });
            TxInstruction {
                index: ix.index,
                program: program_str(&ix.program),
                ix_type: ix_type_str(&ix.instruction_type),
                severity: severity_str(&ix.severity),
                accounts: InstructionAccounts {
                    source: ix.details.get("source").cloned(),
                    destination: ix.details.get("destination").cloned(),
                    authority: ix.details.get("authority").cloned(),
                    from: ix.details.get("from").cloned(),
                    to: ix.details.get("to").cloned(),
                },
                token,
                flags: ix.risk_flags.clone(),
            }
        })
        .collect()
}
pub fn build_balances(
    account_keys: &[String],
    pre_sol: &[u64],
    post_sol: &[u64],
    pre_tokens: &[TokenBalance],
    post_tokens: &[TokenBalance],
) -> TxBalances {
    let mut changes = Vec::new();

    for (i, address) in account_keys.iter().enumerate() {
        let pre_lamports = pre_sol.get(i).copied().unwrap_or(0);
        let post_lamports = post_sol.get(i).copied().unwrap_or(pre_lamports);

        let sol_change_lamports = post_lamports as i64 - pre_lamports as i64;

        let sol = if sol_change_lamports != 0 {
            Some(SolChange {
                pre_lamports,
                post_lamports,
                change_lamports: sol_change_lamports,
                pre_sol: lamports_to_sol(pre_lamports),
                post_sol: lamports_to_sol(post_lamports),
                change_sol: sol_change_lamports as f64 / LAMPORTS_PER_SOL as f64,
            })
        } else {
            None
        };

        let tokens = build_token_changes(address, pre_tokens, post_tokens);

        if sol.is_some() || !tokens.is_empty() {
            changes.push(BalanceChange {
                address: address.clone(),
                sol,
                tokens,
            });
        }
    }

    TxBalances { changes }
}

fn build_token_changes(
    address: &str,
    pre_tokens: &[TokenBalance],
    post_tokens: &[TokenBalance],
) -> Vec<TokenChange> {
    let mut mints = Vec::<String>::new();

    for token in pre_tokens.iter().chain(post_tokens.iter()) {
        if token.address == address && !mints.contains(&token.mint) {
            mints.push(token.mint.clone());
        }
    }

    mints
        .into_iter()
        .filter_map(|mint| {
            let pre = pre_tokens
                .iter()
                .find(|t| t.address == address && t.mint == mint);

            let post = post_tokens
                .iter()
                .find(|t| t.address == address && t.mint == mint);

            let pre_raw = pre
                .map(|t| t.amount.clone())
                .unwrap_or_else(|| "0".to_string());
            let post_raw = post
                .map(|t| t.amount.clone())
                .unwrap_or_else(|| "0".to_string());

            let decimals = pre.or(post).map(|t| t.decimals).unwrap_or(0);

            let pre_i = pre_raw.parse::<i128>().unwrap_or(0);
            let post_i = post_raw.parse::<i128>().unwrap_or(0);
            let diff = post_i - pre_i;

            if diff == 0 {
                return None;
            }

            Some(TokenChange {
                account_index: pre.or(post).and_then(|t| t.account_index),
                mint,
                owner: pre.or(post).and_then(|t| t.owner.clone()),
                program_id: pre.or(post).and_then(|t| t.program_id.clone()),

                pre_raw: pre_raw.clone(),
                post_raw: post_raw.clone(),
                change_raw: if diff >= 0 {
                    format!("+{}", diff)
                } else {
                    diff.to_string()
                },

                pre_ui: pre
                    .and_then(|t| t.ui_amount.clone())
                    .unwrap_or_else(|| raw_to_ui_string(&pre_raw, decimals)),
                post_ui: post
                    .and_then(|t| t.ui_amount.clone())
                    .unwrap_or_else(|| raw_to_ui_string(&post_raw, decimals)),
                change_ui: signed_ui_string(diff, decimals),

                decimals,
            })
        })
        .collect()
}

pub fn build_expected_balances(report: &TransactionReport) -> TxBalances {
    let mut changes = Vec::new();

    for ix in &report.instructions {
        match ix.instruction_type {
            InstructionType::Transfer => {
                if let (Some(from), Some(to), Some(lamports)) = (
                    ix.details.get("from"),
                    ix.details.get("to"),
                    ix.details.get("lamports"),
                ) {
                    if let Ok(lamports) = lamports.parse::<u64>() {
                        let change_sol = lamports as f64 / LAMPORTS_PER_SOL as f64;

                        changes.push(BalanceChange {
                            address: from.clone(),
                            sol: Some(SolChange {
                                pre_lamports: 0,
                                post_lamports: 0,
                                change_lamports: -(lamports as i64),
                                pre_sol: 0.0,
                                post_sol: 0.0,
                                change_sol: -change_sol,
                            }),
                            tokens: vec![],
                        });

                        changes.push(BalanceChange {
                            address: to.clone(),
                            sol: Some(SolChange {
                                pre_lamports: 0,
                                post_lamports: 0,
                                change_lamports: lamports as i64,
                                pre_sol: 0.0,
                                post_sol: 0.0,
                                change_sol,
                            }),
                            tokens: vec![],
                        });
                    }
                }
            }

            InstructionType::TokenTransfer | InstructionType::TokenTransferChecked => {
                if let (Some(source), Some(destination), Some(amount_raw)) = (
                    ix.details.get("source"),
                    ix.details.get("destination"),
                    ix.details
                        .get("amount_raw")
                        .or_else(|| ix.details.get("amount")),
                ) {
                    let mint = ix.details.get("mint").cloned().unwrap_or_default();

                    let decimals = ix
                        .details
                        .get("decimals")
                        .and_then(|d| d.parse::<u8>().ok())
                        .unwrap_or(0);

                    let amount_ui = raw_to_ui_string(amount_raw, decimals);

                    changes.push(BalanceChange {
                        address: source.clone(),
                        sol: None,
                        tokens: vec![TokenChange {
                            account_index: None,
                            mint: mint.clone(),
                            owner: None,
                            program_id: None,
                            pre_raw: "unknown".to_string(),
                            post_raw: "unknown".to_string(),
                            change_raw: format!("-{}", amount_raw),
                            pre_ui: "unknown".to_string(),
                            post_ui: "unknown".to_string(),
                            change_ui: format!("-{}", amount_ui),
                            decimals,
                        }],
                    });

                    changes.push(BalanceChange {
                        address: destination.clone(),
                        sol: None,
                        tokens: vec![TokenChange {
                            account_index: None,
                            mint,
                            owner: None,
                            program_id: None,
                            pre_raw: "unknown".to_string(),
                            post_raw: "unknown".to_string(),
                            change_raw: format!("+{}", amount_raw),
                            pre_ui: "unknown".to_string(),
                            post_ui: "unknown".to_string(),
                            change_ui: format!("+{}", amount_ui),
                            decimals,
                        }],
                    });
                }
            }

            _ => {}
        }
    }

    TxBalances { changes }
}

pub fn extract_cu_budget(report: &TransactionReport) -> Option<u64> {
    report
        .instructions
        .iter()
        .find(|ix| matches!(ix.instruction_type, InstructionType::SetComputeUnitLimit))
        .and_then(|ix| ix.details.get("compute_units"))
        .and_then(|s| s.parse::<u64>().ok())
}

pub fn raw_to_ui_string(raw: &str, decimals: u8) -> String {
    let value = raw.parse::<i128>().unwrap_or(0);
    unsigned_ui_string(value, decimals)
}

fn signed_ui_string(value: i128, decimals: u8) -> String {
    if value >= 0 {
        format!("+{}", unsigned_ui_string(value, decimals))
    } else {
        format!("-{}", unsigned_ui_string(value.abs(), decimals))
    }
}

fn unsigned_ui_string(value: i128, decimals: u8) -> String {
    if decimals == 0 {
        return value.to_string();
    }

    let divisor = 10_i128.pow(decimals as u32);
    let whole = value / divisor;
    let frac = value % divisor;

    if frac == 0 {
        whole.to_string()
    } else {
        let frac = format!("{:0width$}", frac, width = decimals as usize);
        format!("{}.{}", whole, frac.trim_end_matches('0'))
    }
}
