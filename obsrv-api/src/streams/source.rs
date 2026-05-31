// ============================================================
// streams/source.rs — Yellowstone gRPC connection
//
// Establishes a connection to a Yellowstone gRPC endpoint
// (e.g. ParaFi at solana-rpc.parafi.tech:10443).
//
// Returns a GeyserGrpcClient ready for subscription.
// The caller (yellowstone.rs) will call .subscribe() on it
// to get a (sink, stream) pair for sending filter requests
// and receiving transaction updates.
// ============================================================

use anyhow::{Context, Result};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient, Interceptor};

/// Connect to a Yellowstone gRPC endpoint and return the client.
///
/// - `endpoint` — full URL including port, e.g. "https://solana-rpc.parafi.tech:10443"
/// - `x_token`  — optional auth token (ParaFi does not require one)
pub async fn connect(
    endpoint: &str,
    x_token: Option<&str>,
) -> Result<GeyserGrpcClient<impl Interceptor>> {
    tracing::info!(endpoint = %endpoint, "connecting to Yellowstone gRPC");

    let mut builder = GeyserGrpcClient::build_from_shared(endpoint.to_string())
        .context("failed to build gRPC client from endpoint")?;

    // attach auth token if provided
    if let Some(token) = x_token {
        builder = builder
            .x_token(Some(token.to_string()))
            .context("failed to set x-token")?;
    }

    // enable TLS — Yellowstone endpoints use TLS
    let tls_config = ClientTlsConfig::new().with_native_roots();
    builder = builder
        .tls_config(tls_config)
        .context("failed to configure TLS")?;

    let client = builder.connect().await.context("gRPC connect failed")?;

    tracing::info!("connected to Yellowstone gRPC");
    Ok(client)
}
