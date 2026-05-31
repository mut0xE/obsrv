"use client";

import React, { useState, useEffect } from "react";
import { api, ApiError, connectWs } from "@/lib/api";
import { useAlerts } from "./ErrorAlert";
import {
  AddressDisplay,
  PulseDot,
  StaticDot,
  RiskBadge,
} from "@/lib/components";

// ── types ──────────────────────────────────────────────────────────
interface FeedEntry {
  t: string;
  wallet: string;
  program: string;
  risk: number;
  summary: string;
  sev: "critical" | "warning" | "safe" | "info";
  _new?: boolean;
  signature?: string;
}

// ── helpers ────────────────────────────────────────────────────────
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

// Strip per-instruction detail, "Risk score: …", and "Fee payer: …" so the
// live feed shows a clean one-liner instead of a multi-paragraph message.
function cleanSummary(s?: string): string {
  if (!s) return "";
  let out = s;
  const ixIdx = out.search(/Instruction\s+\d+\s*:/i);
  if (ixIdx > 0) out = out.slice(0, ixIdx);
  const riskIdx = out.search(/Risk\s+score\s*:/i);
  if (riskIdx > 0) out = out.slice(0, riskIdx);
  const feeIdx = out.search(/Fee\s+payer\s*:/i);
  if (feeIdx > 0) out = out.slice(0, feeIdx);
  out = out.replace(/^[^\w(]+/, "").replace(/\s+/g, " ").trim();
  if (out.length > 120) out = out.slice(0, 117) + "…";
  return out || "Transaction processed";
}

function sinceFromAddedAt(addedAt?: number) {
  if (!addedAt) return "—";
  const ms = Date.now() - addedAt;
  const h = Math.floor(ms / 3_600_000);
  if (h < 1) return `${Math.floor(ms / 60_000)}m`;
  if (h < 24) return `${h}h ${Math.floor((ms % 3_600_000) / 60_000)}m`;
  const d = Math.floor(h / 24);
  return `${d}d ${h % 24}h`;
}

// ── WALLET MONITOR CARD ────────────────────────────────────────────
function WalletMonitorCard({
  wallet,
  onRemove,
}: {
  wallet: any;
  onRemove: () => void;
}) {
  const active = wallet.status !== "paused";
  const threshold = wallet.alert_threshold ?? 7;
  const alerts24h = wallet.alerts_24h ?? 0;
  const since = sinceFromAddedAt(wallet.added_at);
  const lastAlert = wallet.last_alert;

  return (
    <div className="panel" style={{ padding: 0, opacity: active ? 1 : 0.6 }}>
      <div style={{ padding: "22px 24px 18px" }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 16,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            {active ? (
              <PulseDot color="safe" />
            ) : (
              <StaticDot color="tertiary" />
            )}
            <span
              className="label-strong"
              style={{ color: active ? "var(--safe)" : "var(--text-tertiary)" }}
            >
              {active ? "MONITORING" : "PAUSED"}
            </span>
          </div>
          <button
            className="btn btn-ghost btn-sm"
            style={{ padding: "4px 8px" }}
            onClick={onRemove}
          >
            ×
          </button>
        </div>

        <div style={{ marginBottom: 14 }}>
          <AddressDisplay
            address={wallet.address || wallet.wallet}
            label={wallet.name || wallet.label}
          />
        </div>

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr 1fr",
            borderTop: "1px solid var(--bg-border)",
            paddingTop: 16,
          }}
        >
          <div>
            <div className="label" style={{ marginBottom: 6 }}>
              WATCHING
            </div>
            <div
              className="data"
              style={{ fontSize: 14, color: "var(--text-primary)" }}
            >
              {active ? since : "—"}
            </div>
          </div>
          <div>
            <div className="label" style={{ marginBottom: 6 }}>
              THRESHOLD
            </div>
            <div
              className="data"
              style={{ fontSize: 14, color: "var(--gold)" }}
            >
              {threshold}/10
            </div>
          </div>
          <div>
            <div className="label" style={{ marginBottom: 6 }}>
              24H ALERTS
            </div>
            <div
              className="data"
              style={{
                fontSize: 14,
                color:
                  alerts24h > 0 ? "var(--warning)" : "var(--text-secondary)",
              }}
            >
              {alerts24h}
            </div>
          </div>
        </div>
      </div>

      {lastAlert ? (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "3px 1fr auto",
            gap: 16,
            padding: "16px 24px",
            borderTop: "1px solid var(--bg-border)",
            background: "var(--bg-surface)",
            alignItems: "center",
          }}
        >
          <div
            className={`sev-bar ${lastAlert.sev ?? "warning"}`}
            style={{ height: 32 }}
          />
          <div>
            <div
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: "var(--text-primary)",
                marginBottom: 2,
              }}
            >
              {lastAlert.summary}
            </div>
            <div className="label">{lastAlert.time}</div>
          </div>
          <RiskBadge level={lastAlert.sev ?? "warning"} size="sm">
            {lastAlert.risk}/10
          </RiskBadge>
        </div>
      ) : (
        <div
          style={{
            padding: "16px 24px",
            borderTop: "1px solid var(--bg-border)",
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--text-tertiary)",
            letterSpacing: "0.08em",
          }}
        >
          NO ALERTS · ALL CLEAR
        </div>
      )}
    </div>
  );
}

// ── PROGRAM MONITOR CARD ───────────────────────────────────────────
function ProgramMonitorCard({
  prog,
  onRemove,
}: {
  prog: any;
  onRemove: () => void;
}) {
  const active = prog.status !== "paused";
  const since = sinceFromAddedAt(prog.added_at);
  const watchIx: string[] = prog.watch_ix ?? prog.instructions ?? [];
  const callsHr = prog.calls_per_hour ?? prog.calls_hr ?? 0;
  const failRate = prog.fail_rate ?? 0;
  const lastAlert = prog.last_alert;

  return (
    <div className="panel" style={{ padding: 0, opacity: active ? 1 : 0.6 }}>
      <div style={{ padding: "22px 24px 18px" }}>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: 16,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            {active ? (
              <PulseDot color="safe" />
            ) : (
              <StaticDot color="tertiary" />
            )}
            <span
              className="label-strong"
              style={{ color: active ? "var(--safe)" : "var(--text-tertiary)" }}
            >
              {active ? "MONITORING" : "PAUSED"}
            </span>
          </div>
          <button
            className="btn btn-ghost btn-sm"
            style={{ padding: "4px 8px" }}
            onClick={onRemove}
          >
            ×
          </button>
        </div>

        <div style={{ marginBottom: 12 }}>
          <AddressDisplay
            address={prog.address || prog.program_id}
            label={prog.name || prog.label}
          />
        </div>

        {watchIx.length > 0 && (
          <div
            style={{
              display: "flex",
              flexWrap: "wrap",
              gap: 6,
              marginBottom: 16,
            }}
          >
            {watchIx.map((ix, i) => (
              <span
                key={i}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  color: "var(--text-secondary)",
                  border: "1px solid var(--bg-border-strong)",
                  padding: "3px 8px",
                  background: "var(--bg-surface)",
                }}
              >
                {ix}
              </span>
            ))}
          </div>
        )}

        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr 1fr",
            borderTop: "1px solid var(--bg-border)",
            paddingTop: 16,
          }}
        >
          <div>
            <div className="label" style={{ marginBottom: 6 }}>
              WATCHING
            </div>
            <div
              className="data"
              style={{ fontSize: 14, color: "var(--text-primary)" }}
            >
              {since}
            </div>
          </div>
          <div>
            <div className="label" style={{ marginBottom: 6 }}>
              CALLS / HR
            </div>
            <div
              className="data"
              style={{ fontSize: 14, color: "var(--text-primary)" }}
            >
              {Number(callsHr).toLocaleString()}
            </div>
          </div>
          <div>
            <div className="label" style={{ marginBottom: 6 }}>
              FAIL RATE
            </div>
            <div
              className="data"
              style={{
                fontSize: 14,
                color:
                  failRate > 10
                    ? "var(--critical)"
                    : failRate > 3
                      ? "var(--warning)"
                      : "var(--safe)",
              }}
            >
              {Number(failRate).toFixed(1)}%
            </div>
          </div>
        </div>
      </div>

      {lastAlert ? (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "3px 1fr auto",
            gap: 16,
            padding: "16px 24px",
            borderTop: "1px solid var(--bg-border)",
            background: "var(--bg-surface)",
            alignItems: "center",
          }}
        >
          <div
            className={`sev-bar ${lastAlert.sev ?? "warning"}`}
            style={{ height: 32 }}
          />
          <div>
            <div
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--text-primary)",
                marginBottom: 2,
              }}
            >
              {lastAlert.summary}
            </div>
            <div className="label">{lastAlert.time}</div>
          </div>
          <RiskBadge level={lastAlert.sev ?? "warning"} size="sm">
            {lastAlert.risk}/10
          </RiskBadge>
        </div>
      ) : (
        <div
          style={{
            padding: "16px 24px",
            borderTop: "1px solid var(--bg-border)",
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--text-tertiary)",
            letterSpacing: "0.08em",
          }}
        >
          NO ALERTS · ALL CLEAR
        </div>
      )}
    </div>
  );
}

// ── ADD FORM ───────────────────────────────────────────────────────
function AddMonitorForm({
  mode,
  onAdded,
}: {
  mode: "wallet" | "program";
  onAdded: () => void | Promise<void>;
}) {
  const [address, setAddress] = useState("");
  const [label, setLabel] = useState("");
  const [watchIx, setWatchIx] = useState("");
  const [telegramChatId, setTelegramChatId] = useState("");
  const [threshold, setThreshold] = useState(7);
  const [channels, setChannels] = useState({
    telegram: true,
    email: false,
    webhook: false,
  });
  const [loading, setLoading] = useState(false);
  const { showError, showSuccess } = useAlerts();
  const isProgram = mode === "program";

  async function handleAdd() {
    if (!address.trim()) return;
    setLoading(true);
    try {
      const addr = address.trim();
      if (mode === "wallet") {
        const resp: any = await api.addWallet(
          addr,
          channels.telegram ? telegramChatId.trim() || undefined : undefined,
          threshold,
        );
        showSuccess(resp?.message || "Wallet added to monitor list");
      } else {
        const resp: any = await api.addProgram(addr, label.trim() || undefined);
        showSuccess(resp?.message || "Program added to monitor list");
      }
      setAddress("");
      setLabel("");
      setWatchIx("");
      setTelegramChatId("");
      await onAdded();
    } catch (err) {
      showError(
        err instanceof ApiError ? err.message : `Failed to add ${mode}`,
      );
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="panel" style={{ position: "sticky", top: 0 }}>
      <div className="panel-head">
        <span className="panel-title">
          + New {isProgram ? "program" : "wallet"} monitor
        </span>
      </div>
      <div
        style={{
          padding: 24,
          display: "flex",
          flexDirection: "column",
          gap: 22,
        }}
      >
        <div>
          <div className="label" style={{ marginBottom: 10 }}>
            {isProgram ? "PROGRAM ID" : "WALLET ADDRESS"}
          </div>
          <input
            className="input"
            placeholder={
              isProgram ? "paste program id…" : "paste solana address…"
            }
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAdd()}
          />
        </div>
        <div>
          <div className="label" style={{ marginBottom: 10 }}>
            LABEL (OPTIONAL)
          </div>
          <input
            className="input"
            placeholder={isProgram ? "jupiter v6 · verified" : "treasury · ops"}
            value={label}
            onChange={(e) => setLabel(e.target.value)}
          />
        </div>

        {isProgram && (
          <div>
            <div className="label" style={{ marginBottom: 10 }}>
              INSTRUCTIONS TO WATCH
            </div>
            <input
              className="input"
              placeholder="route, approve, 0x4f2a … (comma separated)"
              value={watchIx}
              onChange={(e) => setWatchIx(e.target.value)}
            />
            <div
              style={{
                marginTop: 10,
                fontFamily: "var(--font-sans)",
                fontSize: 12,
                color: "var(--text-tertiary)",
                lineHeight: 1.5,
              }}
            >
              Leave empty to watch every instruction. obsrv alerts on failure
              spikes and call-rate anomalies per instruction.
            </div>
          </div>
        )}

        {!isProgram && (
          <div>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "baseline",
                marginBottom: 12,
              }}
            >
              <span className="label">RISK THRESHOLD</span>
              <span
                className="data"
                style={{ fontSize: 16, color: "var(--gold)" }}
              >
                {threshold}/10
              </span>
            </div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(6,1fr)",
                gap: 4,
              }}
            >
              {[5, 6, 7, 8, 9, 10].map((n) => (
                <button
                  key={n}
                  onClick={() => setThreshold(n)}
                  style={{
                    padding: "12px 0",
                    fontFamily: "var(--font-mono)",
                    fontSize: 13,
                    background:
                      threshold === n ? "var(--gold-dim)" : "transparent",
                    color:
                      threshold === n ? "var(--gold)" : "var(--text-secondary)",
                    border: `1px solid ${threshold === n ? "var(--gold)" : "var(--bg-border-strong)"}`,
                    fontVariantNumeric: "tabular-nums",
                    transition: "all 120ms",
                    cursor: "pointer",
                  }}
                >
                  {n}
                </button>
              ))}
            </div>
            <div
              style={{
                marginTop: 10,
                fontFamily: "var(--font-sans)",
                fontSize: 12,
                color: "var(--text-tertiary)",
                lineHeight: 1.5,
              }}
            >
              Alert when{" "}
              {threshold >= 8
                ? "only severe risks appear"
                : threshold >= 6
                  ? "warnings and above appear"
                  : "almost any anomaly appears"}
              .
            </div>
          </div>
        )}

        {!isProgram && (
          <div>
            <div className="label" style={{ marginBottom: 12 }}>
              NOTIFY VIA
            </div>
            <div style={{ border: "1px solid var(--bg-border-strong)" }}>
              {[
                { k: "telegram" as const, l: "Telegram" },
                { k: "email" as const, l: "Email" },
                { k: "webhook" as const, l: "Webhook" },
              ].map((c, i, arr) => {
                const enabled = channels[c.k];
                const placeholderByKey: Record<string, string> = {
                  telegram: "@your_chat_id",
                  email: "ops@domain.com",
                  webhook: "POST https://…",
                };
                return (
                  <label
                    key={c.k}
                    style={{
                      display: "grid",
                      gridTemplateColumns: "18px 1fr auto",
                      gap: 12,
                      alignItems: "center",
                      padding: "13px 14px",
                      borderBottom:
                        i < arr.length - 1
                          ? "1px solid var(--bg-border)"
                          : "none",
                      cursor: "pointer",
                      background: enabled ? "var(--bg-surface)" : "transparent",
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={enabled}
                      onChange={() =>
                        setChannels({ ...channels, [c.k]: !enabled })
                      }
                      style={{ accentColor: "var(--safe)" }}
                    />
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--text-primary)",
                      }}
                    >
                      {c.l}
                    </span>
                    {c.k === "telegram" && enabled ? (
                      <input
                        value={telegramChatId}
                        onChange={(e) => setTelegramChatId(e.target.value)}
                        placeholder={placeholderByKey[c.k]}
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          color: "var(--text-secondary)",
                          background: "transparent",
                          border: "none",
                          textAlign: "right",
                          width: 140,
                        }}
                      />
                    ) : (
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          color: "var(--text-tertiary)",
                        }}
                      >
                        {placeholderByKey[c.k]}
                      </span>
                    )}
                  </label>
                );
              })}
            </div>
          </div>
        )}

        <button
          className="btn btn-safe"
          style={{ width: "100%", justifyContent: "center", padding: "14px" }}
          onClick={handleAdd}
          disabled={loading || !address.trim()}
        >
          {loading ? (
            <>
              Adding<span className="blink">_</span>
            </>
          ) : (
            `▸ Start monitoring ${isProgram ? "program" : "wallet"}`
          )}
        </button>
      </div>
    </div>
  );
}

// ── LIVE FEED ──────────────────────────────────────────────────────
function LiveFeed({ mode }: { mode: "wallet" | "program" }) {
  const [feed, setFeed] = useState<FeedEntry[]>([]);
  const [paused, setPaused] = useState(false);
  const [connected, setConnected] = useState(false);
  const pausedRef = React.useRef(paused);
  pausedRef.current = paused;
  const feedBufferRef = React.useRef<any[]>([]);

  useEffect(() => {
    const flush = setInterval(() => {
      if (pausedRef.current || feedBufferRef.current.length === 0) return;
      const toProcess = feedBufferRef.current.splice(0, 5);
      setFeed((prev) =>
        [
          ...toProcess.map((data: any) => {
            const risk = Math.round(
              data.risk ?? data.risk_score ?? data.analysis?.risk_score ?? 0,
            );
            const wallet = data.wallet ?? data.fee_payer ?? "";
            const shortWallet =
              wallet.length > 8
                ? wallet.slice(0, 4) + "…" + wallet.slice(-4)
                : wallet || "—";
            const program =
              data.program_id ??
              data.program ??
              data.analysis?.programs?.[0] ??
              "";
            const shortProgram =
              program.length > 8
                ? program.slice(0, 4) + "…" + program.slice(-4)
                : program || "—";
            return {
              t: nowTime(),
              wallet: shortWallet,
              program: shortProgram,
              risk,
              summary: cleanSummary(
                data.summary ??
                  data.analysis?.summary ??
                  data.description,
              ),
              sev: sevFromRisk(risk),
              _new: true,
              signature: data.signature,
            } as FeedEntry;
          }),
          ...prev,
        ].slice(0, 16),
      );
    }, 800);
    return () => clearInterval(flush);
  }, []);

  useEffect(() => {
    let ws: WebSocket | null = null;
    try {
      ws = connectWs((data: any) => {
        const isEvent =
          data?.signature ||
          data?.type === "tx_processed" ||
          data?.type === "alert" ||
          data?.type === "transaction";
        if (!isEvent) return;
        const hasWallet = data.wallet || data.fee_payer;
        const hasProgram =
          data.program_id ||
          data.program ||
          data.analysis?.programs?.length > 0;
        if (mode === "wallet" && !hasWallet) return;
        if (mode === "program" && !hasProgram) return;
        feedBufferRef.current.push(data);
      });
      ws.onopen = () => setConnected(true);
      ws.onclose = () => setConnected(false);
    } catch {}
    return () => ws?.close();
  }, [mode]);

  return (
    <div className="panel">
      <div className="panel-head">
        <PulseDot color={connected ? "safe" : "critical"} />
        <span className="panel-title">Live activity</span>
        <span className="panel-sub">
          — {connected ? "websocket connected" : "connecting…"}
        </span>
        <div
          style={{
            marginLeft: "auto",
            display: "flex",
            gap: 8,
            alignItems: "center",
          }}
        >
          <span className="label">{feed.length} events</span>
          <button
            className="btn btn-ghost btn-sm"
            onClick={() => setPaused(!paused)}
          >
            {paused ? "▶ Resume" : "⏸ Pause"}
          </button>
        </div>
      </div>
      {feed.length === 0 ? (
        <div
          style={{
            padding: "52px 28px",
            textAlign: "center",
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--text-tertiary)",
            letterSpacing: "0.1em",
          }}
        >
          WAITING FOR TRANSACTIONS<span className="blink">_</span>
        </div>
      ) : (
        <div style={{ maxHeight: 380, overflowY: "auto" }}>
          {feed.map((e, i) => (
            <div
              key={`${e.t}-${i}`}
              className={i === 0 && e._new ? "entry-enter" : ""}
              style={{
                display: "grid",
                gridTemplateColumns: "3px 96px 120px 84px 1fr",
                gap: 18,
                padding: "15px 28px",
                borderBottom: "1px solid var(--bg-border)",
                alignItems: "center",
              }}
            >
              <div className={`sev-bar ${e.sev}`} style={{ height: 18 }} />
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--text-tertiary)",
                }}
              >
                {e.t}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--text-primary)",
                }}
              >
                {e.wallet}
              </span>
              <RiskBadge level={e.sev} size="sm">
                {e.risk}/10
              </RiskBadge>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--text-primary)",
                }}
              >
                {e.summary}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── MAIN ───────────────────────────────────────────────────────────
export function PageMonitor() {
  const [mode, setMode] = useState<"wallet" | "program">("wallet");
  const [loading, setLoading] = useState(false);
  const { showError, showSuccess } = useAlerts();

  useEffect(() => {
    fetchList();
    // Poll the DB every 5s so newly-added wallets/programs (and external
    // changes) appear without a manual refresh.
    const id = setInterval(() => fetchList(), 5000);
    return () => clearInterval(id);
  }, []);

  const [wallets, setWallets] = useState<any[]>([]);
  const [programs, setPrograms] = useState<any[]>([]);

  async function fetchList() {
    setLoading(true);
    try {
      const data = await api.getMonitorList();
      setWallets(data?.wallets ?? []);
      setPrograms(data?.programs ?? []);
    } catch (err) {
      const message =
        err instanceof ApiError ? err.message : "Failed to fetch monitor list";
      const detail =
        err instanceof ApiError && err.data?.timeout
          ? "Please ensure the obsrv API server is running on localhost:3001"
          : undefined;
      showError(message, detail);
    } finally {
      setLoading(false);
    }
  }

  async function handleWalletAdded() {
    await fetchList();
  }

  async function handleProgramAdded() {
    await fetchList();
  }

  async function handleRemoveWallet(address: string) {
    try {
      await api.removeWallet(address);
      showSuccess("Wallet removed");
      await fetchList();
    } catch (err) {
      showError(
        err instanceof ApiError ? err.message : "Failed to remove wallet",
      );
    }
  }

  async function handleRemoveProgram(address: string) {
    try {
      await api.removeProgram(address);
      showSuccess("Program removed");
      await fetchList();
    } catch (err) {
      showError(
        err instanceof ApiError ? err.message : "Failed to remove program",
      );
    }
  }
  const activeWallets = wallets.filter(
    (w: any) => w.status !== "paused",
  ).length;
  const activePrograms = programs.filter(
    (p: any) => p.status !== "paused",
  ).length;
  const isProgram = mode === "program";

  return (
    <>
      <div className="page-head">
        <div className="page-head-row">
          <div>
            <div className="label">MONITOR</div>
            <h1>Watch wallets and programs in real time</h1>
            <p>
              obsrv subscribes to each tracked wallet and program, scoring every
              transaction the instant it confirms. Cross your threshold and you
              get pinged before the damage is done.
            </p>
          </div>
          <div style={{ display: "flex", gap: 10 }}>
            <button
              className="btn btn-ghost btn-sm"
              disabled
              style={{ display: "inline-flex", alignItems: "center", gap: 8, opacity: 0.7, cursor: "not-allowed" }}
              title="Telegram bot — coming soon"
            >
              Telegram bot
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 9,
                  letterSpacing: "0.14em",
                  color: "var(--gold)",
                  background: "var(--gold-dim)",
                  border: "1px solid var(--gold)",
                  padding: "2px 6px",
                  textTransform: "uppercase",
                }}
              >
                Upcoming
              </span>
            </button>
            <button className="btn btn-ghost btn-sm">Export rules</button>
          </div>
        </div>
      </div>

      {/* segmented toggle */}
      <div
        style={{
          padding: "20px 48px 0",
          display: "flex",
          gap: 0,
          borderBottom: "1px solid var(--bg-border)",
        }}
      >
        {[
          ["wallet", "WALLETS", wallets.length] as const,
          ["program", "PROGRAM IDS", programs.length] as const,
        ].map(([k, l, n]) => (
          <div
            key={k}
            onClick={() => setMode(k)}
            style={{
              padding: "14px 26px",
              marginBottom: -1,
              cursor: "pointer",
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              letterSpacing: "0.12em",
              color:
                mode === k ? "var(--text-primary)" : "var(--text-secondary)",
              borderBottom: `2px solid ${mode === k ? "var(--gold)" : "transparent"}`,
              display: "flex",
              alignItems: "center",
              gap: 10,
            }}
          >
            {l}
            <span
              style={{
                fontSize: 11,
                color: "var(--text-tertiary)",
                background: "var(--bg-surface)",
                border: "1px solid var(--bg-border)",
                padding: "1px 7px",
              }}
            >
              {n}
            </span>
          </div>
        ))}
      </div>

      <div className="page-body">
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 340px",
            gap: 40,
            alignItems: "start",
          }}
        >
          <div>
            <div className="section" style={{ marginTop: 0 }}>
              <div className="section-head">
                <div>
                  <div className="label">
                    {isProgram
                      ? `${activePrograms} PROGRAMS WATCHED`
                      : `${activeWallets} ACTIVE · ${wallets.length - activeWallets} PAUSED`}
                  </div>
                  <h2>{isProgram ? "Watched programs" : "Watched wallets"}</h2>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <button
                    className="btn btn-ghost btn-sm"
                    style={{ background: "var(--bg-surface)" }}
                  >
                    All
                  </button>
                  <button className="btn btn-ghost btn-sm">Active</button>
                  <button
                    className="btn btn-ghost btn-sm"
                    onClick={fetchList}
                    disabled={loading}
                    title="Re-fetch from DB"
                  >
                    {loading ? <>↻<span className="blink">_</span></> : "↻ Refresh"}
                  </button>
                </div>
              </div>

              {loading ? (
                <div
                  style={{
                    color: "var(--text-tertiary)",
                    fontFamily: "var(--font-mono)",
                    fontSize: 13,
                  }}
                >
                  Loading<span className="blink">_</span>
                </div>
              ) : isProgram ? (
                programs.length > 0 ? (
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "1fr 1fr",
                      gap: 16,
                    }}
                  >
                    {programs.map((p: any, i: number) => (
                      <ProgramMonitorCard
                        key={i}
                        prog={p}
                        onRemove={() =>
                          handleRemoveProgram(p.address || p.program_id)
                        }
                      />
                    ))}
                  </div>
                ) : (
                  <div
                    className="panel"
                    style={{
                      padding: "48px 32px",
                      textAlign: "center",
                      fontFamily: "var(--font-mono)",
                      fontSize: 12,
                      color: "var(--text-tertiary)",
                      letterSpacing: "0.1em",
                    }}
                  >
                    NO PROGRAMS MONITORED YET · ADD ONE →
                  </div>
                )
              ) : wallets.length > 0 ? (
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 16,
                  }}
                >
                  {wallets.map((w: any, i: number) => (
                    <WalletMonitorCard
                      key={i}
                      wallet={w}
                      onRemove={() => handleRemoveWallet(w.address || w.wallet)}
                    />
                  ))}
                </div>
              ) : (
                <div
                  className="panel"
                  style={{
                    padding: "48px 32px",
                    textAlign: "center",
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                    color: "var(--text-tertiary)",
                    letterSpacing: "0.1em",
                  }}
                >
                  NO WALLETS MONITORED YET · ADD ONE →
                </div>
              )}
            </div>

            <div className="section">
              <div className="section-head">
                <div>
                  <div className="label">STREAM</div>
                  <h2>Live activity feed</h2>
                </div>
              </div>
              <LiveFeed mode={mode} />
            </div>
          </div>

          <AddMonitorForm
            mode={mode}
            onAdded={mode === "wallet" ? handleWalletAdded : handleProgramAdded}
          />
        </div>
      </div>
    </>
  );
}
