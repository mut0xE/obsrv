"use client";

import React, { useEffect, useState } from "react";
import {
  connectWallet,
  disconnectWallet,
  hasWalletProvider,
  onAuthChange,
  readWalletAuth,
  type WalletAuth,
} from "@/lib/wallet-auth";
import { useAlerts } from "./ErrorAlert";

export function WalletConnect() {
  const [auth, setAuth] = useState<WalletAuth | null>(null);
  const [busy, setBusy] = useState(false);
  const [hasProvider, setHasProvider] = useState(true);
  const { showError, showSuccess } = useAlerts();

  useEffect(() => {
    setAuth(readWalletAuth());
    setHasProvider(hasWalletProvider());
    return onAuthChange(() => {
      setAuth(readWalletAuth());
      setHasProvider(hasWalletProvider());
    });
  }, []);

  async function handleConnect() {
    setBusy(true);
    try {
      const a = await connectWallet();
      showSuccess(`Connected ${a.pubkey.slice(0, 4)}…${a.pubkey.slice(-4)}`);
    } catch (err: any) {
      showError(err?.message || "Failed to connect wallet");
    } finally {
      setBusy(false);
    }
  }

  async function handleDisconnect() {
    setBusy(true);
    try {
      await disconnectWallet();
    } finally {
      setBusy(false);
    }
  }

  if (!auth) {
    return (
      <div style={{ padding: "0 12px" }}>
        <button
          className="btn"
          style={{
            width: "100%",
            justifyContent: "center",
            padding: "11px 12px",
            background: "var(--gold-dim)",
            border: "1px solid var(--gold)",
            color: "var(--gold)",
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            letterSpacing: "0.12em",
          }}
          onClick={handleConnect}
          disabled={busy || !hasProvider}
          title={
            hasProvider
              ? "Connect a Solana wallet to sign in"
              : "Install Phantom or any Solana wallet"
          }
        >
          {busy ? (
            <>
              Connecting<span className="blink">_</span>
            </>
          ) : hasProvider ? (
            "▸ Connect wallet"
          ) : (
            "Wallet not found"
          )}
        </button>
        {!hasProvider && (
          <div
            style={{
              marginTop: 8,
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              color: "var(--text-tertiary)",
              lineHeight: 1.5,
              textAlign: "center",
            }}
          >
            Install{" "}
            <a
              href="https://phantom.app"
              target="_blank"
              rel="noreferrer"
              style={{ color: "var(--gold)" }}
            >
              phantom.app
            </a>
          </div>
        )}
      </div>
    );
  }

  const short = `${auth.pubkey.slice(0, 4)}…${auth.pubkey.slice(-4)}`;

  return (
    <div style={{ padding: "0 12px" }}>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "auto 1fr auto",
          alignItems: "center",
          gap: 10,
          padding: "10px 12px",
          border: "1px solid var(--bg-border-strong)",
          background: "var(--bg-surface)",
        }}
      >
        <span
          style={{
            width: 8,
            height: 8,
            background: "var(--safe)",
            boxShadow: "0 0 8px var(--safe)",
          }}
        />
        <div style={{ minWidth: 0 }}>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              letterSpacing: "0.12em",
              textTransform: "uppercase",
              color: "var(--text-tertiary)",
            }}
          >
            Signed in
          </div>
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--text-primary)",
              marginTop: 2,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
            title={auth.pubkey}
          >
            {short}
          </div>
        </div>
        <button
          className="btn btn-ghost btn-sm"
          style={{ padding: "4px 8px", fontSize: 11 }}
          onClick={handleDisconnect}
          disabled={busy}
          title="Disconnect wallet"
        >
          ⏏
        </button>
      </div>
    </div>
  );
}
