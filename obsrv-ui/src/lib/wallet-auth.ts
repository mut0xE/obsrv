"use client";

// ────────────────────────────────────────────────────────────────────────
// Wallet-signed authentication.
//
// User flow:
//   1. User clicks "Connect wallet" → `window.solana.connect()`
//   2. We ask the wallet to sign a one-time challenge string
//      ("obsrv auth · sign to prove you own this wallet · <ts>").
//   3. We persist `{ pubkey, message, signature }` in localStorage.
//   4. Every API request carries three headers the backend can verify
//      with `ed25519-dalek`:
//          X-User-Id        — base58 wallet pubkey  (used as the user id)
//          X-User-Auth-Msg  — the exact bytes that were signed
//          X-User-Signature — base58 of the 64-byte signature
//
// Backend changes (recommended):
//   • Read those three headers in a middleware, verify with ed25519-dalek
//     once per session, and stamp `user_id = pubkey` on every row written
//     by `/monitor/wallet` and `/monitor/program`.
//   • `/monitor/list` then `WHERE user_id = $1` — proper isolation.
// ────────────────────────────────────────────────────────────────────────

const STORAGE_KEY = "obsrv:wallet-auth";

export interface WalletAuth {
  pubkey: string;
  message: string;
  signature: string;
  connectedAt: number;
}

interface SolanaProvider {
  isPhantom?: boolean;
  publicKey?: { toString(): string } | null;
  connect: (opts?: { onlyIfTrusted?: boolean }) => Promise<{
    publicKey: { toString(): string };
  }>;
  disconnect: () => Promise<void>;
  signMessage: (
    msg: Uint8Array,
    encoding?: string,
  ) => Promise<{ signature: Uint8Array; publicKey?: { toString(): string } }>;
}

function getProvider(): SolanaProvider | null {
  if (typeof window === "undefined") return null;
  // Phantom + most other wallets inject `window.solana`.
  const w = window as unknown as { solana?: SolanaProvider };
  return w.solana ?? null;
}

export function hasWalletProvider(): boolean {
  return !!getProvider();
}

export function readWalletAuth(): WalletAuth | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as WalletAuth;
    if (!parsed.pubkey || !parsed.message || !parsed.signature) return null;
    return parsed;
  } catch {
    return null;
  }
}

function writeWalletAuth(auth: WalletAuth | null) {
  if (typeof window === "undefined") return;
  try {
    if (auth) {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(auth));
    } else {
      window.localStorage.removeItem(STORAGE_KEY);
    }
  } catch {
    /* quota / private mode */
  }
  // notify any in-page listeners (wallet connect button updates immediately)
  window.dispatchEvent(new CustomEvent("obsrv:wallet-auth"));
}

// base58 — minimal implementation (no extra dep). Lifted from bitcoinjs.
const B58_ALPHABET =
  "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
function bytesToBase58(bytes: Uint8Array): string {
  if (bytes.length === 0) return "";
  let zeros = 0;
  while (zeros < bytes.length && bytes[zeros] === 0) zeros++;
  const digits: number[] = [];
  for (let i = zeros; i < bytes.length; i++) {
    let carry = bytes[i];
    for (let j = 0; j < digits.length; j++) {
      carry += digits[j] << 8;
      digits[j] = carry % 58;
      carry = (carry / 58) | 0;
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = (carry / 58) | 0;
    }
  }
  let out = "";
  for (let i = 0; i < zeros; i++) out += "1";
  for (let i = digits.length - 1; i >= 0; i--) out += B58_ALPHABET[digits[i]];
  return out;
}

export async function connectWallet(): Promise<WalletAuth> {
  const provider = getProvider();
  if (!provider) {
    throw new Error(
      "No Solana wallet found. Install Phantom (phantom.app) and reload.",
    );
  }

  const { publicKey } = await provider.connect();
  const pubkey = publicKey.toString();

  const message = `obsrv auth · sign to prove you own this wallet · ${Date.now()}`;
  const encoded = new TextEncoder().encode(message);

  const { signature } = await provider.signMessage(encoded, "utf8");
  const sigB58 = bytesToBase58(signature);

  const auth: WalletAuth = {
    pubkey,
    message,
    signature: sigB58,
    connectedAt: Date.now(),
  };
  writeWalletAuth(auth);
  return auth;
}

export async function disconnectWallet(): Promise<void> {
  const provider = getProvider();
  try {
    await provider?.disconnect();
  } catch {
    /* ignore */
  }
  writeWalletAuth(null);
}

/**
 * Subscribe to auth changes (connect, disconnect, cross-tab updates).
 * Returns an unsubscribe function.
 */
export function onAuthChange(cb: () => void): () => void {
  if (typeof window === "undefined") return () => {};
  const handler = () => cb();
  window.addEventListener("obsrv:wallet-auth", handler);
  window.addEventListener("storage", handler);
  return () => {
    window.removeEventListener("obsrv:wallet-auth", handler);
    window.removeEventListener("storage", handler);
  };
}
