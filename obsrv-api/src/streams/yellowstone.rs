// ============================================================
// streams/yellowstone.rs — main gRPC streaming loop
//
// This is the heart of obsrv's real-time monitoring system.
// It connects to a Yellowstone gRPC endpoint (e.g. ParaFi),
// subscribes to transactions touching watched wallets/programs,
// and processes each transaction through the analytics pipeline.
//
// Architecture:
//   1. Connect to Yellowstone gRPC (source.rs)
//   2. Query DB for watched addresses (filters.rs)
//   3. Build a SubscribeRequest and send it
//   4. Process incoming transactions (processor.rs)
//   5. Every 30s, check if watched addresses changed and re-subscribe
//   6. On disconnect, reconnect with exponential backoff
//
// The stream runs as a background tokio task spawned from main.rs.
// It is fully independent of the HTTP server.
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use sqlx::PgPool;
use tokio::sync::broadcast;
use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;

use super::{filters, processor, source};
use crate::{config::Config, ws::WsEvent};

/// Entry point: run the gRPC stream forever with automatic reconnection.
///
/// This function never returns under normal operation.
/// On disconnect or error, it reconnects with exponential backoff.
pub async fn run_stream(config: Arc<Config>, pool: PgPool, ws_tx: broadcast::Sender<WsEvent>) {
    let mut backoff = Duration::from_secs(1);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    loop {
        match run_stream_inner(&config, &pool, &ws_tx).await {
            Ok(()) => {
                // clean exit (shouldn't happen normally) — reset backoff
                backoff = Duration::from_secs(1);
                tracing::info!("stream ended cleanly, reconnecting");
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    backoff_secs = backoff.as_secs(),
                    "stream disconnected, reconnecting"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
}

/// Single stream session: connect → subscribe → process until error.
async fn run_stream_inner(
    config: &Config,
    pool: &PgPool,
    ws_tx: &broadcast::Sender<WsEvent>,
) -> Result<()> {
    // ============================================================
    // STEP 1: Connect to Yellowstone gRPC
    // ============================================================

    let mut client = source::connect(&config.grpc_endpoint, config.grpc_x_token.as_deref())
        .await
        .context("failed to connect to Yellowstone gRPC")?;

    // ============================================================
    // STEP 2: Build initial subscription from watched addresses
    // ============================================================

    let current_addresses = wait_for_addresses(pool).await?;

    let initial_request = filters::build_subscribe_request(&current_addresses)
        .context("no addresses to watch (should not happen after wait_for_addresses)")?;

    tracing::info!(
        address_count = current_addresses.len(),
        "subscribing to transaction stream"
    );

    // ============================================================
    // STEP 3: Subscribe — get (sink, stream) pair
    // ============================================================

    let (mut subscribe_tx, mut stream) = client
        .subscribe_with_request(Some(initial_request))
        .await
        .context("gRPC subscribe failed")?;

    tracing::info!("subscription active, processing transactions");

    // ============================================================
    // STEP 4: Spawn address-change detector
    //
    // Every 30s, re-query the DB for watched addresses.
    // If the set changed, send a new SubscribeRequest through the sink.
    // Yellowstone supports mid-stream resubscription.
    // ============================================================

    let pool_clone = pool.clone();
    let (addr_change_tx, mut addr_change_rx) = tokio::sync::mpsc::channel::<Vec<String>>(1);

    let address_checker = tokio::spawn({
        let mut known = current_addresses.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.tick().await; // skip first immediate tick

            loop {
                interval.tick().await;

                match filters::collect_watched_addresses(&pool_clone).await {
                    Ok(fresh) if fresh != known => {
                        tracing::info!(
                            old_count = known.len(),
                            new_count = fresh.len(),
                            "watched addresses changed, triggering resubscription"
                        );
                        known = fresh.clone();
                        if addr_change_tx.send(fresh).await.is_err() {
                            break; // main loop dropped the receiver
                        }
                    }
                    Ok(_) => {} // no change
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to check watched addresses");
                    }
                }
            }
        }
    });

    // ============================================================
    // STEP 5: Main processing loop
    //
    // Process incoming SubscribeUpdate messages.
    // Also check for address changes to resubscribe.
    // ============================================================

    loop {
        tokio::select! {
            // new message from the gRPC stream
            msg = stream.next() => {
                match msg {
                    Some(Ok(update)) => {
                        match update.update_oneof {
                            Some(UpdateOneof::Transaction(tx_update)) => {
                                if let Some(ref tx_info) = tx_update.transaction {
                                    let signature = bs58::encode(&tx_info.signature).into_string();

                                    // extract transaction and meta from the protobuf
                                    if let (Some(proto_tx), Some(meta)) =
                                        (&tx_info.transaction, &tx_info.meta)
                                    {
                                        processor::process_transaction(
                                            meta,
                                            proto_tx,
                                            &signature,
                                            tx_update.slot,
                                            pool,
                                            ws_tx,
                                            &config.telegram_bot_token,
                                        )
                                        .await;
                                    }
                                }
                            }
                            Some(UpdateOneof::Ping(_)) => {
                                // respond with pong to keep connection alive
                                use futures_util::SinkExt;
                                use yellowstone_grpc_proto::geyser::SubscribeRequest;

                                let pong = SubscribeRequest {
                                    ping: Some(yellowstone_grpc_proto::geyser::SubscribeRequestPing { id: 1 }),
                                    ..Default::default()
                                };
                                if let Err(e) = subscribe_tx.send(pong).await {
                                    tracing::warn!(error = %e, "failed to send pong");
                                }
                            }
                            Some(UpdateOneof::Pong(_)) => {
                                // pong response received, nothing to do
                            }
                            _ => {
                                // ignore slot/block/account updates
                            }
                        }
                    }
                    Some(Err(e)) => {
                        address_checker.abort();
                        return Err(anyhow::anyhow!("stream error: {}", e));
                    }
                    None => {
                        address_checker.abort();
                        return Err(anyhow::anyhow!("stream closed by server"));
                    }
                }
            }

            // address set changed — send new subscription
            Some(new_addresses) = addr_change_rx.recv() => {
                if let Some(request) = filters::build_subscribe_request(&new_addresses) {
                    use futures_util::SinkExt;
                    tracing::info!(
                        count = new_addresses.len(),
                        "resubscribing with updated addresses"
                    );
                    if let Err(e) = subscribe_tx.send(request).await {
                        address_checker.abort();
                        return Err(anyhow::anyhow!("failed to resubscribe: {}", e));
                    }
                    let _ = new_addresses; // consumed by build_subscribe_request
                }
            }
        }
    }
}

/// Wait until there is at least one watched address in the DB.
/// Polls every 10 seconds. This prevents subscribing with an empty filter.
async fn wait_for_addresses(pool: &PgPool) -> Result<Vec<String>> {
    loop {
        let addresses = filters::collect_watched_addresses(pool)
            .await
            .context("failed to query watched addresses")?;

        if !addresses.is_empty() {
            return Ok(addresses);
        }

        tracing::info!("no watched addresses yet, waiting 10s before retry");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}
