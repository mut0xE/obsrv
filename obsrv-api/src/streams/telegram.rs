// ============================================================
// streams/telegram.rs — Telegram Bot alert sender
//
// Sends formatted alert messages to Telegram when a watched
// wallet's transaction exceeds the risk threshold.
//
// Uses the Telegram Bot API (sendMessage with MarkdownV2).
// Requires TELEGRAM_BOT_TOKEN env var to be set.
// The chat_id comes from the watched_wallet's telegram_chat_id.
// ============================================================

use obsrv_core::types::WalletAlert;

/// Send an alert message to Telegram.
///
/// Silently logs errors — alert delivery is best-effort.
/// The bot token is validated at startup; if missing, this
/// function is never called.
pub async fn send_alert(bot_token: &str, alert: &WalletAlert) {
    let chat_id = &alert.telegram_chat_id;

    let sol_summary: String = alert
        .sol_changes
        .iter()
        .map(|s| {
            let sign = if s.change_sol >= 0.0 { "+" } else { "" };
            format!(
                "  {} {}{:.6} SOL",
                s.address[..8].to_string(),
                sign,
                s.change_sol
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let token_summary: String = alert
        .token_changes
        .iter()
        .take(5)
        .map(|t| {
            format!(
                "  {} {} (mint: {}..)",
                t.change_ui,
                t.mint[..8].to_string(),
                &t.mint[..8]
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let flags_text = if alert.flags.is_empty() {
        "None".to_string()
    } else {
        alert
            .flags
            .iter()
            .map(|f| format!("  - {}", f))
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Build plain text message (Telegram parse_mode not used to avoid escaping issues)
    let text = format!(
        "🚨 OBSRV ALERT — High Risk Transaction\n\
         \n\
         Wallet: {wallet}\n\
         Risk: {risk}/10 ({level})\n\
         Signature: {sig}\n\
         Slot: {slot}\n\
         Fee: {fee} lamports\n\
         CU: {cu}\n\
         \n\
         Summary:\n{summary}\n\
         \n\
         Flags:\n{flags}\n\
         {sol_section}\
         {token_section}\
         \n\
         View on Solscan: https://solscan.io/tx/{sig}",
        wallet = &alert.wallet,
        risk = alert.risk_score,
        level = &alert.risk_level,
        sig = &alert.signature,
        slot = alert.slot,
        fee = alert.fee_lamports,
        cu = alert
            .cu_consumed
            .map(|c| c.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        summary = &alert.summary,
        flags = flags_text,
        sol_section = if sol_summary.is_empty() {
            String::new()
        } else {
            format!("\nSOL Changes:\n{}\n", sol_summary)
        },
        token_section = if token_summary.is_empty() {
            String::new()
        } else {
            format!("\nToken Changes:\n{}\n", token_summary)
        },
    );

    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "disable_web_page_preview": true,
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                tracing::info!(
                    wallet = %alert.wallet,
                    chat_id = %chat_id,
                    "telegram alert sent"
                );
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                tracing::warn!(
                    wallet = %alert.wallet,
                    chat_id = %chat_id,
                    status = %status,
                    body = %body,
                    "telegram alert failed"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                wallet = %alert.wallet,
                chat_id = %chat_id,
                error = %e,
                "telegram alert request failed"
            );
        }
    }
}
