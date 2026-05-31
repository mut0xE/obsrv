"use client";

import { create } from "zustand";
import { api, connectWs } from "./api";

export type FeedKind = "wallet" | "program";

export interface FeedEntry {
  t: string;
  wallet: string;
  walletFull: string;
  program: string;
  programFull: string;
  risk: number;
  summary: string;
  sev: "critical" | "warning" | "safe" | "info";
  signature?: string;
  kind: FeedKind;
  /** address that matched a monitored entity (wallet or program id) */
  matchedAddress?: string;
  /** label assigned to that monitored entity, if any */
  matchedLabel?: string;
}

interface ObsrvStore {
  // ── monitor list ────────────────────────────────────────────────
  wallets: any[];
  programs: any[];
  monitorLoading: boolean;
  fetchMonitorList: () => Promise<void>;

  // ── live feed (split by kind so wallet/program pages can scope) ─
  walletFeed: FeedEntry[];
  programFeed: FeedEntry[];
  paused: boolean;
  setPaused: (v: boolean) => void;

  // ── websocket ───────────────────────────────────────────────────
  wsConnected: boolean;
  wsStarted: boolean;
  startWs: () => void;
}

function sevFromRisk(risk: number): FeedEntry["sev"] {
  if (risk >= 8) return "critical";
  if (risk >= 5) return "warning";
  if (risk >= 3) return "info";
  return "safe";
}

function nowTime() {
  const d = new Date();
  return [d.getHours(), d.getMinutes(), d.getSeconds()]
    .map((n) => String(n).padStart(2, "0"))
    .join(":");
}

function cleanSummary(s?: string): string {
  if (!s) return "Transaction processed";
  let out = s;
  const ixIdx = out.search(/Instruction\s+\d+\s*:/i);
  if (ixIdx > 0) out = out.slice(0, ixIdx);
  const riskIdx = out.search(/Risk\s+score\s*:/i);
  if (riskIdx > 0) out = out.slice(0, riskIdx);
  const feeIdx = out.search(/Fee\s+payer\s*:/i);
  if (feeIdx > 0) out = out.slice(0, feeIdx);
  out = out
    .replace(/^[^\w(⚠ℹ✓✗]+/u, "")
    .replace(/\s+/g, " ")
    .trim();
  if (out.length > 140) out = out.slice(0, 137) + "…";
  return out || "Transaction processed";
}

function shortAddr(s: string): string {
  if (!s) return "—";
  return s.length > 8 ? `${s.slice(0, 4)}…${s.slice(-4)}` : s;
}

export const useObsrv = create<ObsrvStore>((set, get) => ({
  wallets: [],
  programs: [],
  monitorLoading: false,

  walletFeed: [],
  programFeed: [],
  paused: false,

  wsConnected: false,
  wsStarted: false,

  setPaused: (v) => set({ paused: v }),

  fetchMonitorList: async () => {
    set({ monitorLoading: true });
    try {
      const data: any = await api.getMonitorList();
      set({
        wallets: data?.wallets ?? [],
        programs: data?.programs ?? [],
      });
    } catch {
      /* leave existing state */
    } finally {
      set({ monitorLoading: false });
    }
  },

  startWs: () => {
    if (get().wsStarted) return;
    set({ wsStarted: true });
    try {
      const ws = connectWs((data: any) => {
        if (get().paused) return;

        // The backend (`obsrv-api/src/ws/mod.rs`) tags every event with
        // `type`. We care about transaction events; ignore everything else.
        const t = data?.type;
        if (t !== "tx_processed" && t !== "alert" && !data?.signature) return;

        // StreamTxEvent shape (obsrv-core::types::StreamTxEvent):
        //   fee_payer, matched_wallets[], matched_programs[],
        //   programs_called[], risk_score, summary, signature
        const walletFull: string = data.fee_payer ?? "";
        const matchedWallets: string[] = Array.isArray(data.matched_wallets)
          ? data.matched_wallets
          : [];
        const matchedPrograms: string[] = Array.isArray(data.matched_programs)
          ? data.matched_programs
          : [];

        const { wallets, programs } = get();

        // Drop the event entirely if it doesn't touch anything *this* user
        // monitors. The backend broadcast is global (per-user fan-out is
        // intentionally done on the client) so events for other users'
        // monitors are filtered out here.
        const myMatchedWallet = wallets.find((w: any) =>
          matchedWallets.includes(w.address || w.wallet),
        );
        const myMatchedProgram = programs.find((p: any) =>
          matchedPrograms.includes(p.address || p.program_id),
        );
        if (!myMatchedWallet && !myMatchedProgram) return;

        const risk = Math.round(data.risk_score ?? 0);
        const baseEntry: FeedEntry = {
          t: nowTime(),
          wallet: shortAddr(walletFull),
          walletFull,
          program: shortAddr(matchedPrograms[0] ?? ""),
          programFull: matchedPrograms[0] ?? "",
          risk,
          summary: cleanSummary(data.summary),
          sev: sevFromRisk(risk),
          signature: data.signature,
          kind: "wallet" as FeedKind,
        };

        if (myMatchedWallet) {
          const addr = myMatchedWallet.address || myMatchedWallet.wallet;
          set((s) => ({
            walletFeed: [
              {
                ...baseEntry,
                kind: "wallet" as FeedKind,
                matchedAddress: addr,
                matchedLabel: myMatchedWallet.name ?? myMatchedWallet.label,
              },
              ...s.walletFeed,
            ].slice(0, 50),
          }));
        }

        if (myMatchedProgram) {
          const pid =
            myMatchedProgram.address || myMatchedProgram.program_id;
          set((s) => ({
            programFeed: [
              {
                ...baseEntry,
                kind: "program" as FeedKind,
                program: shortAddr(pid),
                programFull: pid,
                matchedAddress: pid,
                matchedLabel: myMatchedProgram.name ?? myMatchedProgram.label,
              },
              ...s.programFeed,
            ].slice(0, 50),
          }));
        }
      });
      ws.onopen = () => set({ wsConnected: true });
      ws.onclose = () => set({ wsConnected: false });
    } catch {
      set({ wsConnected: false });
    }
  },
}));
