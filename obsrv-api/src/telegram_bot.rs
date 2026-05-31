// ============================================================
// telegram_bot.rs — @obsrv_mon_bot via teloxide
//
// Long-polling bot that guides users through monitoring setup.
// Commands:
//   /start          — Welcome + instructions
//   /add <wallet> [threshold] — Add wallet to monitor
//   /remove <wallet> — Stop monitoring
//   /list           — Show monitored wallets for this chat
//   /chatid         — Show chat ID
//   /help           — Show commands
// ============================================================

use sqlx::PgPool;
use std::sync::Arc;
use teloxide::{prelude::*, respond, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "obsrv monitor bot commands:")]
enum Command {
    #[command(description = "Welcome and setup instructions")]
    Start,
    #[command(description = "Show available commands")]
    Help,
    #[command(description = "Show your Telegram chat ID")]
    ChatId,
    #[command(description = "Add wallet: /add <address> [threshold]")]
    Add(String),
    #[command(description = "Remove wallet: /remove <address>")]
    Remove(String),
    #[command(description = "List your monitored wallets")]
    List,
}

pub async fn run_bot(bot_token: String, pool: PgPool) {
    tracing::info!("telegram bot starting (@obsrv_mon_bot)");

    let bot = Bot::new(bot_token);
    let pool = Arc::new(pool);

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
            let pool = pool.clone();
            async move {
                handle_command(bot, msg, cmd, &pool).await?;
                respond(())
            }
        });

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    pool: &PgPool,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id;
    let text = match cmd {
        Command::Start => cmd_start(chat_id.0),
        Command::Help => cmd_help(),
        Command::ChatId => format!("Your chat ID: `{}`", chat_id.0),
        Command::Add(args) => cmd_add(pool, chat_id.0, &args).await,
        Command::Remove(args) => cmd_remove(pool, chat_id.0, &args).await,
        Command::List => cmd_list(pool, chat_id.0).await,
    };

    bot.send_message(chat_id, text).await?;
    Ok(())
}

fn cmd_start(chat_id: i64) -> String {
    format!(
        "Welcome to obsrv monitor bot!\n\n\
         I'll send you real-time alerts when your watched Solana wallets \
         execute high-risk transactions.\n\n\
         Your chat ID: {}\n\n\
         Quick start:\n\
         /add <wallet_address> — Start monitoring (threshold 7)\n\
         /add <wallet_address> 5 — Monitor with custom threshold\n\
         /list — Show your monitored wallets\n\
         /remove <wallet_address> — Stop monitoring\n\n\
         Default alert threshold: 7/10 (only critical alerts).\n\
         Lower the number to receive more alerts.",
        chat_id
    )
}

fn cmd_help() -> String {
    "/add <wallet> [threshold] — Add wallet to monitor\n  \
       threshold: 1-10 (default 7, lower = more alerts)\n\n\
     /remove <wallet> — Stop monitoring a wallet\n\n\
     /list — Show all your monitored wallets\n\n\
     /chatid — Show your Telegram chat ID\n\n\
     /help — Show this message\n\n\
     When a monitored wallet executes a transaction with risk >= your threshold, \
     you'll get an alert with risk score, balance changes, programs called, and a Solscan link."
        .to_string()
}

async fn cmd_add(pool: &PgPool, chat_id: i64, args: &str) -> String {
    let parts: Vec<&str> = args.trim().split_whitespace().collect();

    if parts.is_empty() || parts[0].is_empty() {
        return "Usage: /add <wallet_address> [threshold]\n\n\
                Example:\n/add 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU\n\
                /add 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU 5"
            .to_string();
    }

    let wallet = parts[0];

    if wallet.len() < 32 || wallet.len() > 44 {
        return "Invalid wallet address. Must be 32-44 characters base58.".to_string();
    }

    let valid_base58 = wallet
        .chars()
        .all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c));
    if !valid_base58 {
        return "Invalid wallet address. Contains non-base58 characters.".to_string();
    }

    let threshold: i32 = if parts.len() >= 2 {
        parts[1].parse().unwrap_or(7).clamp(1, 10)
    } else {
        7
    };

    let chat_id_str = chat_id.to_string();

    let result = sqlx::query(
        "INSERT INTO watched_wallets (wallet, telegram_chat_id, alert_threshold, active, created_at)
         VALUES ($1, $2, $3, TRUE, EXTRACT(EPOCH FROM NOW())::BIGINT)
         ON CONFLICT (wallet) DO UPDATE SET
            telegram_chat_id = $2,
            alert_threshold = $3,
            active = TRUE",
    )
    .bind(wallet)
    .bind(&chat_id_str)
    .bind(threshold)
    .execute(pool)
    .await;

    match result {
        Ok(_) => format!(
            "Monitoring wallet:\n{}\n\nAlert threshold: {}/10\n\
             You'll receive alerts here when risk >= {}.",
            wallet, threshold, threshold
        ),
        Err(e) => {
            tracing::error!(error = %e, "telegram bot: failed to add wallet");
            "Failed to add wallet. Please try again.".to_string()
        }
    }
}

async fn cmd_remove(pool: &PgPool, chat_id: i64, args: &str) -> String {
    let wallet = args.trim();

    if wallet.is_empty() {
        return "Usage: /remove <wallet_address>".to_string();
    }

    let chat_id_str = chat_id.to_string();

    let result = sqlx::query(
        "UPDATE watched_wallets SET active = FALSE
         WHERE wallet = $1 AND telegram_chat_id = $2 AND active = TRUE",
    )
    .bind(wallet)
    .bind(&chat_id_str)
    .execute(pool)
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                format!(
                    "Stopped monitoring:\n{}\n\nHistorical data preserved.",
                    wallet
                )
            } else {
                "Wallet not found in your monitors. Use /list to see your wallets.".to_string()
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "telegram bot: failed to remove wallet");
            "Failed to remove wallet. Please try again.".to_string()
        }
    }
}

async fn cmd_list(pool: &PgPool, chat_id: i64) -> String {
    let chat_id_str = chat_id.to_string();

    let rows = sqlx::query_as::<_, (String, i32)>(
        "SELECT wallet, alert_threshold FROM watched_wallets
         WHERE telegram_chat_id = $1 AND active = TRUE
         ORDER BY created_at DESC",
    )
    .bind(&chat_id_str)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(wallets) => {
            if wallets.is_empty() {
                "No wallets being monitored.\n\nUse /add <wallet_address> to start.".to_string()
            } else {
                let mut msg = format!("Your monitored wallets ({}):\n\n", wallets.len());
                for (wallet, threshold) in &wallets {
                    msg.push_str(&format!(
                        "  {}... (threshold: {})\n",
                        &wallet[..8],
                        threshold
                    ));
                    msg.push_str(&format!("  {}\n\n", wallet));
                }
                msg
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "telegram bot: failed to list wallets");
            "Failed to fetch wallet list. Please try again.".to_string()
        }
    }
}
