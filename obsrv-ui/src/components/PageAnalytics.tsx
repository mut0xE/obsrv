"use client";

import React, { useState, useEffect } from "react";
import { api, ApiError } from "@/lib/api";
import { useAlerts } from "./ErrorAlert";
import {
  AddressDisplay,
  ProgramPill,
  RiskBadge,
  PulseDot,
  StatCard,
} from "@/lib/components";

// ── helpers ────────────────────────────────────────────────────────
function programType(
  name: string,
): "system" | "token" | "compute" | "serum" | "unknown" {
  const n = (name || "").toLowerCase();
  if (n.includes("compute")) return "compute";
  if (n.includes("token") || n.includes("spl")) return "token";
  if (n.includes("system") || n.includes("1111")) return "system";
  if (
    n.includes("jupiter") ||
    n.includes("serum") ||
    n.includes("mango") ||
    n.includes("drift") ||
    n.includes("kamino") ||
    n.includes("marinade") ||
    n.includes("openbook")
  )
    return "serum";
  return "unknown";
}

function fmt(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
  return String(n);
}

// Inline horizontal bar for success/fail split
function SplitBar({
  success,
  failed,
  total,
}: {
  success: number;
  failed: number;
  total: number;
}) {
  const sPct = total ? (success / total) * 100 : 0;
  const fPct = total ? (failed / total) * 100 : 0;
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
      <div
        style={{
          flex: 1,
          height: 6,
          background: "var(--bg-elevated)",
          position: "relative",
          display: "flex",
        }}
      >
        <div
          style={{
            width: sPct + "%",
            height: "100%",
            background: "var(--safe)",
            transition: "width 600ms",
          }}
        />
        <div
          style={{
            width: fPct + "%",
            height: "100%",
            background: "var(--critical)",
            transition: "width 600ms",
          }}
        />
      </div>
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          color: "var(--safe)",
          minWidth: 36,
        }}
      >
        {success.toLocaleString()}
      </span>
      <span style={{ color: "var(--text-tertiary)", fontSize: 11 }}>/</span>
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          color: "var(--critical)",
        }}
      >
        {failed.toLocaleString()}
      </span>
    </div>
  );
}

// Mini spark bars for trend
function TrendBars({ data, color }: { data: number[]; color: string }) {
  const max = Math.max(...data, 1);
  return (
    <div
      style={{ display: "flex", gap: 2, alignItems: "flex-end", height: 22 }}
    >
      {data.map((v, i) => (
        <div
          key={i}
          style={{
            width: 5,
            height: Math.max(2, (v / max) * 22),
            background: color,
            opacity: 0.7 + (i / data.length) * 0.3,
          }}
        />
      ))}
    </div>
  );
}

// ── WALLET TAB ─────────────────────────────────────────────────────
function WalletTab() {
  const [wallet, setWallet] = useState("");
  const [inputVal, setInputVal] = useState("");
  const [loading, setLoading] = useState(false);
  const [data, setData] = useState<any>(null);
  const [monitoredWallets, setMonitoredWallets] = useState<string[]>([]);
  const [sortMetric, setSortMetric] = useState<"calls" | "cu" | "failures">(
    "calls",
  );
  const { showError } = useAlerts();

  async function runAnalysis(addr: string, silent = false) {
    if (!addr.trim()) return;
    setLoading(true);
    setData(null);
    try {
      const result = await api.getWalletAnalytics(addr.trim());
      setData(result);
      setWallet(addr.trim());
    } catch (err) {
      if (!silent) {
        showError(
          err instanceof ApiError
            ? err.message
            : "Failed to fetch wallet analytics",
        );
      }
    } finally {
      setLoading(false);
    }
  }

  // Pull monitored wallets from the DB and auto-pick the first one so the
  // dashboard renders against real data instead of a hardcoded sample.
  useEffect(() => {
    (async () => {
      try {
        const list = await api.getMonitorList();
        const addrs: string[] = (list?.wallets ?? [])
          .map((w: any) => w.address || w.wallet)
          .filter(Boolean);
        setMonitoredWallets(addrs);
        if (addrs.length > 0) {
          setInputVal(addrs[0]);
          runAnalysis(addrs[0], true);
        }
      } catch {
        /* silent — page stays at empty input */
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const summary = data?.wallet_stats ?? data?.summary ?? {};
  const programs: any[] = data?.program_stats ?? data?.programs ?? [];
  const instructions: any[] =
    data?.instruction_stats ?? data?.instructions ?? [];

  // tracked check
  const isTracked = !!wallet;

  return (
    <div>
      {/* input bar */}
      <div
        className="panel"
        style={{
          display: "grid",
          gridTemplateColumns: "1fr auto auto",
          alignItems: "stretch",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            padding: "16px 24px",
            gap: 16,
            borderRight: "1px solid var(--bg-border)",
          }}
        >
          <span className="label">WALLET</span>
          <input
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runAnalysis(inputVal)}
            placeholder="paste wallet address…"
            style={{
              flex: 1,
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--text-primary)",
              background: "transparent",
              border: "none",
              outline: "none",
            }}
          />
        </div>
        {isTracked && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              padding: "0 24px",
              borderRight: "1px solid var(--bg-border)",
            }}
          >
            <span className="label" style={{ color: "var(--safe)" }}>
              TRACKED
            </span>
          </div>
        )}
        <button
          className="btn btn-primary"
          style={{ borderRadius: 0, border: 0, padding: "0 28px" }}
          onClick={() => runAnalysis(inputVal)}
          disabled={loading || !inputVal.trim()}
        >
          {loading ? (
            <>
              Analyzing<span className="blink">_</span>
            </>
          ) : (
            "RUN ANALYSIS"
          )}
        </button>
      </div>

      {/* Quick-pick chips for every monitored wallet in the DB */}
      {monitoredWallets.length > 0 && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 12, alignItems: "center" }}>
          <span className="label" style={{ marginRight: 6 }}>MONITORED:</span>
          {monitoredWallets.map((addr) => {
            const active = addr === wallet;
            return (
              <button
                key={addr}
                onClick={() => { setInputVal(addr); runAnalysis(addr); }}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  padding: "4px 10px",
                  background: active ? "var(--gold-dim)" : "var(--bg-surface)",
                  color: active ? "var(--gold)" : "var(--text-secondary)",
                  border: `1px solid ${active ? "var(--gold)" : "var(--bg-border-strong)"}`,
                  cursor: "pointer",
                }}
              >
                {addr.slice(0, 4)}…{addr.slice(-4)}
              </button>
            );
          })}
        </div>
      )}

      {loading && (
        <div
          className="panel"
          style={{ padding: "80px 32px", textAlign: "center", marginTop: 16 }}
        >
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              letterSpacing: "0.3em",
              color: "var(--info)",
            }}
          >
            LOADING<span className="blink">_</span>
          </div>
        </div>
      )}

      {data && (
        <>
          {/* summary strip */}
          <div className="section" style={{ marginTop: 32 }}>
            <div className="section-head">
              <div>
                <div className="label">SUMMARY · 30D</div>
                <h2>Wallet activity</h2>
              </div>
            </div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(4, 1fr)",
                gap: 0,
              }}
            >
              {[
                {
                  label: "TOTAL TRANSACTIONS",
                  value: (summary.total_txs ?? 0).toLocaleString(),
                  sub: `≈ ${Math.round((summary.total_txs ?? 0) / 30)} per day`,
                  color: "var(--text-primary)",
                },
                {
                  label: "SUCCESSFUL",
                  value: (summary.success_txs ?? 0).toLocaleString(),
                  sub: `${summary.success_rate ? summary.success_rate.toFixed(1) : "0"}% success`,
                  color: "var(--safe)",
                },
                {
                  label: "FAILED",
                  value: (summary.failed_txs ?? 0).toLocaleString(),
                  sub: `${summary.failed_txs && summary.total_txs ? ((summary.failed_txs / summary.total_txs) * 100).toFixed(1) : "0"}% failed`,
                  color: "var(--critical)",
                },
                {
                  label: "TOTAL CU",
                  value: ((summary.total_cu ?? 0) / 1000000).toFixed(2) + "M",
                  sub: `Avg ${((summary.avg_cu ?? 0) / 1000).toFixed(0)}K per tx`,
                  color: "var(--text-primary)",
                },
              ].map((item, i) => (
                <div
                  key={i}
                  className="panel"
                  style={{
                    borderRight: i < 3 ? "none" : undefined,
                    padding: "28px 28px 24px",
                  }}
                >
                  <div className="label" style={{ marginBottom: 16 }}>
                    {item.label}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--font-data)",
                      fontSize: 32,
                      color: item.color,
                      fontVariantNumeric: "tabular-nums",
                      lineHeight: 1,
                    }}
                  >
                    {item.value}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--text-tertiary)",
                      marginTop: 10,
                    }}
                  >
                    {item.sub}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* programs table */}
          {programs.length > 0 && (
            <div className="section">
              <div className="section-head">
                <div>
                  <div className="label">PROGRAMS</div>
                  <h2>Most-called programs</h2>
                </div>
                <div style={{ display: "flex", gap: 8 }}>
                  {(["calls", "cu", "failures"] as const).map((m) => (
                    <button
                      key={m}
                      className={`btn btn-ghost btn-sm ${sortMetric === m ? "active" : ""}`}
                      style={{
                        background:
                          sortMetric === m
                            ? "var(--bg-surface)"
                            : "transparent",
                        borderColor:
                          sortMetric === m
                            ? "var(--bg-border-strong)"
                            : "transparent",
                      }}
                      onClick={() => setSortMetric(m)}
                    >
                      {m === "calls" ? "CALLS ▾" : m.toUpperCase()}
                    </button>
                  ))}
                </div>
              </div>
              <div className="panel">
                <table className="tbl">
                  <thead>
                    <tr>
                      <th>PROGRAM</th>
                      <th className="num">TOTAL CALLS</th>
                      <th>SUCCESS / FAILED</th>
                      <th className="num">SUCCESS %</th>
                      <th className="num">AVG CU</th>
                      <th>TREND · 7D</th>
                    </tr>
                  </thead>
                  <tbody>
                    {[...programs]
                      .sort((a, b) => {
                        if (sortMetric === "calls")
                          return (b.call_count ?? 0) - (a.call_count ?? 0);
                        if (sortMetric === "cu")
                          return (b.avg_cu ?? 0) - (a.avg_cu ?? 0);
                        return (b.failed_count ?? 0) - (a.failed_count ?? 0);
                      })
                      .map((prog: any, i: number) => {
                        const total = prog.call_count ?? 0;
                        const success = prog.success_count ?? 0;
                        const failed = prog.failed_count ?? 0;
                        const pct = total ? (success / total) * 100 : 100;
                        const avgCu = prog.avg_cu ?? 0;
                        const trend =
                          prog.trend_7d ??
                          [3, 4, 5, 4, 6, 5, 7].map((v) =>
                            Math.round(v * (total / 30)),
                          );
                        const trendColor =
                          failed / total > 0.1
                            ? "var(--critical)"
                            : "var(--safe)";
                        return (
                          <tr key={i} className="row-hover">
                            <td>
                              <ProgramPill
                                name={
                                  prog.name || prog.program_name || "Unknown"
                                }
                                type={programType(prog.name || "")}
                              />
                            </td>
                            <td
                              className="num"
                              style={{ fontVariantNumeric: "tabular-nums" }}
                            >
                              {total.toLocaleString()}
                            </td>
                            <td>
                              <SplitBar
                                success={success}
                                failed={failed}
                                total={total}
                              />
                            </td>
                            <td
                              className="num"
                              style={{
                                color:
                                  pct < 90
                                    ? "var(--critical)"
                                    : pct < 98
                                      ? "var(--warning)"
                                      : "var(--safe)",
                              }}
                            >
                              {pct.toFixed(1)}%
                            </td>
                            <td
                              className="num"
                              style={{ fontVariantNumeric: "tabular-nums" }}
                            >
                              {avgCu.toLocaleString()}
                            </td>
                            <td>
                              <TrendBars
                                data={
                                  Array.isArray(trend)
                                    ? trend
                                    : [1, 1, 1, 1, 1, 1, 1]
                                }
                                color={trendColor}
                              />
                            </td>
                          </tr>
                        );
                      })}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* instructions frequency */}
          {instructions.length > 0 && (
            <div className="section">
              <div className="section-head">
                <div>
                  <div className="label">INSTRUCTIONS</div>
                  <h2>Frequency &amp; failure rate</h2>
                </div>
              </div>
              <div className="panel">
                {instructions.map((ix: any, i: number) => {
                  const count = ix.call_count ?? 0;
                  const maxCount = instructions[0]?.call_count ?? 1;
                  const failed = ix.failed_count ?? 0;
                  const failRate = count > 0 ? failed / count : 0;
                  const barPct = (count / maxCount) * 100;
                  return (
                    <div
                      key={i}
                      style={{
                        display: "grid",
                        gridTemplateColumns: "260px 1fr 60px 60px",
                        gap: 20,
                        padding: "13px 28px",
                        borderBottom:
                          i < instructions.length - 1
                            ? "1px solid var(--bg-border)"
                            : "none",
                        alignItems: "center",
                      }}
                    >
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 13,
                          color: "var(--text-primary)",
                        }}
                      >
                        {ix.instruction_type ?? ix.type ?? ix.name}
                      </span>
                      <div
                        style={{
                          position: "relative",
                          height: 6,
                          background: "var(--bg-elevated)",
                        }}
                      >
                        <div
                          style={{
                            position: "absolute",
                            left: 0,
                            top: 0,
                            height: "100%",
                            width: barPct + "%",
                            background:
                              failRate > 0.1
                                ? "var(--critical)"
                                : "var(--info)",
                            transition: "width 600ms",
                          }}
                        />
                      </div>
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 12,
                          color: "var(--text-secondary)",
                          textAlign: "right",
                          fontVariantNumeric: "tabular-nums",
                        }}
                      >
                        {count.toLocaleString()}
                      </span>
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 12,
                          textAlign: "right",
                          color:
                            failRate > 0.1
                              ? "var(--critical)"
                              : failRate > 0.02
                                ? "var(--warning)"
                                : "var(--text-tertiary)",
                        }}
                      >
                        {(failRate * 100).toFixed(1)}%
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── PROGRAM TAB ────────────────────────────────────────────────────
function ProgramTab() {
  const [inputVal, setInputVal] = useState("");
  const [activeProgram, setActiveProgram] = useState("");
  const [loading, setLoading] = useState(false);
  const [data, setData] = useState<any>(null);
  const [monitoredPrograms, setMonitoredPrograms] = useState<
    { id: string; name?: string }[]
  >([]);
  const [topPrograms, setTopPrograms] = useState<any[]>([]);
  const [sortMetric, setSortMetric] = useState<"calls" | "cu" | "failures">(
    "calls",
  );
  const { showError } = useAlerts();

  async function runAnalysis(addr: string, silent = false) {
    if (!addr.trim()) return;
    setLoading(true);
    setData(null);
    try {
      const result = await api.getProgramAnalytics(addr.trim());
      setData(result);
      setActiveProgram(addr.trim());
    } catch (err) {
      if (!silent) {
        showError(
          err instanceof ApiError
            ? err.message
            : "Failed to fetch program analytics",
        );
      }
    } finally {
      setLoading(false);
    }
  }

  // Pull monitored programs + the network-wide top programs from the DB.
  useEffect(() => {
    (async () => {
      try {
        const [list, top] = await Promise.all([
          api.getMonitorList().catch(() => ({ programs: [] })),
          api.getTopPrograms(20).catch(() => [] as any[]),
        ]);
        const mon = ((list as any)?.programs ?? []).map((p: any) => ({
          id: p.program_id || p.address,
          name: p.name || p.label,
        })).filter((p: any) => !!p.id);
        setMonitoredPrograms(mon);
        setTopPrograms(top || []);
        const first = mon[0]?.id || (top && top[0]?.program_id);
        if (first) {
          setInputVal(first);
          runAnalysis(first, true);
        }
      } catch {
        /* silent */
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const summary = data?.summary ?? {};
  const instructions: any[] = data?.instructions ?? [];
  const isVerified = data?.verified ?? false;

  return (
    <div>
      {/* input bar */}
      <div
        className="panel"
        style={{
          display: "grid",
          gridTemplateColumns: "1fr auto auto",
          alignItems: "stretch",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            padding: "16px 24px",
            gap: 16,
            borderRight: "1px solid var(--bg-border)",
          }}
        >
          <span className="label">PROGRAM</span>
          <input
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runAnalysis(inputVal)}
            placeholder="paste program id…"
            style={{
              flex: 1,
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--text-primary)",
              background: "transparent",
              border: "none",
              outline: "none",
            }}
          />
        </div>
        {data && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              padding: "0 24px",
              borderRight: "1px solid var(--bg-border)",
            }}
          >
            <span
              className="label"
              style={{ color: isVerified ? "var(--safe)" : "var(--warning)" }}
            >
              {isVerified ? "VERIFIED" : "UNVERIFIED"}
            </span>
          </div>
        )}
        <button
          className="btn btn-primary"
          style={{ borderRadius: 0, border: 0, padding: "0 28px" }}
          onClick={() => runAnalysis(inputVal)}
          disabled={loading || !inputVal.trim()}
        >
          {loading ? (
            <>
              Analyzing<span className="blink">_</span>
            </>
          ) : (
            "RUN ANALYSIS"
          )}
        </button>
      </div>

      {/* Quick-pick chips for every monitored program in the DB */}
      {monitoredPrograms.length > 0 && (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 12, alignItems: "center" }}>
          <span className="label" style={{ marginRight: 6 }}>MONITORED:</span>
          {monitoredPrograms.map((p) => {
            const active = p.id === activeProgram;
            return (
              <button
                key={p.id}
                onClick={() => { setInputVal(p.id); runAnalysis(p.id); }}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  padding: "4px 10px",
                  background: active ? "var(--gold-dim)" : "var(--bg-surface)",
                  color: active ? "var(--gold)" : "var(--text-secondary)",
                  border: `1px solid ${active ? "var(--gold)" : "var(--bg-border-strong)"}`,
                  cursor: "pointer",
                }}
              >
                {p.name || `${p.id.slice(0, 4)}…${p.id.slice(-4)}`}
              </button>
            );
          })}
        </div>
      )}

      {/* Network-wide top programs (from /analytics/programs) — visible
          regardless of any input, so the page always shows real DB data. */}
      {topPrograms.length > 0 && (
        <div className="section">
          <div className="section-head">
            <div>
              <div className="label">NETWORK · TOP PROGRAMS</div>
              <h2>Most-used programs across all tracked transactions</h2>
            </div>
          </div>
          <div className="panel">
            <table className="tbl">
              <thead>
                <tr>
                  <th style={{ width: 50 }}>#</th>
                  <th>Program</th>
                  <th className="num">Total calls</th>
                  <th className="num">Failed</th>
                  <th className="num">Avg CU</th>
                  <th className="num">Unique callers</th>
                </tr>
              </thead>
              <tbody>
                {topPrograms.map((p: any, i: number) => {
                  const total = p.total_calls ?? p.call_count ?? 0;
                  const failed = p.failed_calls ?? p.failed_count ?? 0;
                  const failPct = total > 0 ? (failed / total) * 100 : 0;
                  const avgCu = p.avg_cu ?? p.avg_compute ?? 0;
                  const callers =
                    p.unique_callers ??
                    p.distinct_callers ??
                    p.unique_signers ??
                    p.caller_count ??
                    p.signers ??
                    0;
                  const id = p.program_id || p.address;
                  return (
                    <tr key={i} className="row-hover" onClick={() => id && (setInputVal(id), runAnalysis(id))} style={{ cursor: id ? "pointer" : "default" }}>
                      <td><span style={{ fontFamily: "var(--font-mono)", color: i === 0 ? "var(--gold)" : "var(--text-tertiary)" }}>{String(i + 1).padStart(2, "0")}</span></td>
                      <td>
                        <ProgramPill name={p.name || (id ? `${id.slice(0, 4)}…${id.slice(-4)}` : "Unknown")} type={programType(p.name || "")} />
                      </td>
                      <td className="num">{total.toLocaleString()}</td>
                      <td className="num"><span style={{ color: failed > 0 ? "var(--critical)" : "var(--text-tertiary)" }}>{failed.toLocaleString()}</span> <span style={{ color: failPct > 1 ? "var(--warning)" : "var(--text-tertiary)", marginLeft: 6, fontSize: 11 }}>({failPct.toFixed(1)}%)</span></td>
                      <td className="num">{Number(avgCu).toLocaleString()}</td>
                      <td className="num">
                        {callers == null ? (
                          <span style={{ color: "var(--text-tertiary)" }}>—</span>
                        ) : (
                          Number(callers).toLocaleString()
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {loading && (
        <div
          className="panel"
          style={{ padding: "80px 32px", textAlign: "center", marginTop: 16 }}
        >
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              letterSpacing: "0.3em",
              color: "var(--info)",
            }}
          >
            LOADING<span className="blink">_</span>
          </div>
        </div>
      )}

      {data && (
        <>
          <div className="section" style={{ marginTop: 32 }}>
            <div className="section-head">
              <div>
                <div className="label">SUMMARY · 7D</div>
                <h2>{data.name || data.program_name || "Program"}</h2>
              </div>
            </div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(4, 1fr)",
                gap: 0,
              }}
            >
              {[
                {
                  label: "TOTAL CALLS",
                  value: fmt(summary.total_calls ?? 0),
                  sub: `${instructions.length} instructions`,
                  color: "var(--text-primary)",
                },
                {
                  label: "SUCCESSFUL",
                  value: fmt(summary.success_calls ?? 0),
                  sub: `${summary.success_rate ? summary.success_rate.toFixed(1) : "0"}% success`,
                  color: "var(--safe)",
                },
                {
                  label: "FAILED",
                  value: fmt(summary.failed_calls ?? 0),
                  sub: `${summary.failed_calls && summary.total_calls ? ((summary.failed_calls / summary.total_calls) * 100).toFixed(1) : "0"}% failed`,
                  color: "var(--critical)",
                },
                {
                  label: "AVG CU",
                  value: fmt(Math.round((summary.avg_cu ?? 0) / 1000)) + "K",
                  sub: `Peak ${fmt(Math.round((summary.peak_cu ?? 0) / 1000))}K`,
                  color: "var(--text-primary)",
                },
              ].map((item, i) => (
                <div
                  key={i}
                  className="panel"
                  style={{
                    borderRight: i < 3 ? "none" : undefined,
                    padding: "28px 28px 24px",
                  }}
                >
                  <div className="label" style={{ marginBottom: 16 }}>
                    {item.label}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--font-data)",
                      fontSize: 32,
                      color: item.color,
                      fontVariantNumeric: "tabular-nums",
                      lineHeight: 1,
                    }}
                  >
                    {item.value}
                  </div>
                  {item.sub && (
                    <div
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        color: "var(--text-tertiary)",
                        marginTop: 10,
                      }}
                    >
                      {item.sub}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {instructions.length > 0 && (
            <div className="section">
              <div className="section-head">
                <div>
                  <div className="label">INSTRUCTIONS · RANKED</div>
                  <h2>Ordered by selected metric</h2>
                </div>
                <div style={{ display: "flex", gap: 8 }}>
                  {(["calls", "cu", "failures"] as const).map((m) => (
                    <button
                      key={m}
                      className="btn btn-ghost btn-sm"
                      style={{
                        background:
                          sortMetric === m
                            ? "var(--bg-surface)"
                            : "transparent",
                        borderColor:
                          sortMetric === m
                            ? "var(--bg-border-strong)"
                            : "transparent",
                      }}
                      onClick={() => setSortMetric(m)}
                    >
                      {m === "calls" ? "CALLS ▾" : m.toUpperCase()}
                    </button>
                  ))}
                </div>
              </div>
              <div className="panel">
                <table className="tbl">
                  <thead>
                    <tr>
                      <th style={{ width: 36 }}>#</th>
                      <th>INSTRUCTION</th>
                      <th className="num">CALLS · 7D</th>
                      <th className="num">SUCCESSFUL</th>
                      <th className="num">FAILED</th>
                      <th className="num">FAIL %</th>
                      <th className="num">CALLERS</th>
                    </tr>
                  </thead>
                  <tbody>
                    {[...instructions]
                      .sort((a, b) => {
                        if (sortMetric === "calls")
                          return (b.call_count ?? 0) - (a.call_count ?? 0);
                        if (sortMetric === "cu")
                          return (b.avg_cu ?? 0) - (a.avg_cu ?? 0);
                        return (b.failed_count ?? 0) - (a.failed_count ?? 0);
                      })
                      .map((ix: any, i: number) => {
                        const count = ix.call_count ?? 0;
                        const success = ix.success_count ?? 0;
                        const failed = ix.failed_count ?? 0;
                        const failPct = count ? (failed / count) * 100 : 0;
                        return (
                          <tr key={i} className="row-hover">
                            <td
                              style={{
                                fontFamily: "var(--font-mono)",
                                fontSize: 12,
                                color: "var(--gold)",
                              }}
                            >
                              {String(i + 1).padStart(2, "0")}
                            </td>
                            <td
                              style={{
                                fontFamily: "var(--font-mono)",
                                fontSize: 13,
                              }}
                            >
                              {ix.instruction_type ?? ix.name ?? ix.type}
                            </td>
                            <td
                              className="num"
                              style={{ fontVariantNumeric: "tabular-nums" }}
                            >
                              {count.toLocaleString()}
                            </td>
                            <td
                              className="num"
                              style={{
                                color: "var(--safe)",
                                fontVariantNumeric: "tabular-nums",
                              }}
                            >
                              {success.toLocaleString()}
                            </td>
                            <td
                              className="num"
                              style={{
                                color: "var(--critical)",
                                fontVariantNumeric: "tabular-nums",
                              }}
                            >
                              {failed.toLocaleString()}
                            </td>
                            <td
                              className="num"
                              style={{
                                color:
                                  failPct > 5
                                    ? "var(--critical)"
                                    : failPct > 1
                                      ? "var(--warning)"
                                      : "var(--text-tertiary)",
                              }}
                            >
                              {failPct.toFixed(2)}%
                            </td>
                            <td
                              className="num"
                              style={{ fontVariantNumeric: "tabular-nums" }}
                            >
                              {(() => {
                                const c =
                                  ix.unique_callers ??
                                  ix.distinct_callers ??
                                  ix.unique_signers ??
                                  ix.caller_count ??
                                  ix.signers;
                                return c == null ? (
                                  <span style={{ color: "var(--text-tertiary)" }}>—</span>
                                ) : (
                                  Number(c).toLocaleString()
                                );
                              })()}
                            </td>
                          </tr>
                        );
                      })}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── SPIKES TAB ─────────────────────────────────────────────────────
// Each "spike" = one instruction belonging to a monitored program, ranked by
// failure rate. baseline = successful_count (calls that ran cleanly),
// today = total call_count (real calls observed), factor = today / baseline.
// All data comes from /monitor/list + /analytics/program?program_id=X.
type SpikeRow = {
  ix: string;
  program: string;
  programId: string;
  baseline: number;
  today: number;
  factor: number;
  level: "critical" | "warning" | "info";
  watched: true;
};

function SpikesTab({ spikeCount }: { spikeCount: number }) {
  const [monitoredPrograms, setMonitoredPrograms] = useState<
    { id: string; name?: string }[]
  >([]);
  const [spikes, setSpikes] = useState<SpikeRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [countdown, setCountdown] = useState(60);

  async function fetchSpikes() {
    setLoading(true);
    try {
      // 1. Get monitored programs from DB.
      const list = await api.getMonitorList();
      const mon: { id: string; name?: string }[] = ((list as any)?.programs ?? [])
        .map((p: any) => ({
          id: p.program_id || p.address,
          name: p.name || p.label,
        }))
        .filter((p: { id?: string }) => !!p.id);
      setMonitoredPrograms(mon);

      // 2. For each monitored program, fetch its instruction stats from DB.
      const perProgram = await Promise.all(
        mon.map((p) =>
          api
            .getProgramAnalytics(p.id)
            .then((res: any) => ({ p, res }))
            .catch(() => ({ p, res: null })),
        ),
      );

      // 3. Flatten into instruction rows.
      const rows: SpikeRow[] = [];
      for (const { p, res } of perProgram) {
        const ixs: any[] = (res as any)?.instructions ?? [];
        for (const ix of ixs) {
          const today = Number(ix.call_count ?? ix.count ?? 0);
          const failed = Number(ix.failed_count ?? 0);
          const baseline = Math.max(0, today - failed);
          if (today === 0) continue;
          const failPct = today > 0 ? failed / today : 0;
          const level: SpikeRow["level"] =
            failPct >= 0.15 ? "critical" : failPct >= 0.05 ? "warning" : "info";
          const factor = baseline > 0 ? today / baseline : today > 0 ? today : 1;
          rows.push({
            ix: ix.instruction_type ?? ix.name ?? ix.type ?? "unknown",
            program: p.name || p.id,
            programId: p.id,
            baseline,
            today,
            factor,
            level,
            watched: true,
          });
        }
      }

      // 4. Rank by factor desc (highest spike on top).
      rows.sort((a, b) => b.factor - a.factor);
      setSpikes(rows);
    } catch {
      /* silent — empty state will show */
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchSpikes();
    const interval = setInterval(() => {
      setCountdown((c) => {
        if (c <= 1) {
          fetchSpikes();
          return 60;
        }
        return c - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const critCount = spikes.filter((s) => s.level === "critical").length;
  const warnCount = spikes.filter((s) => s.level === "warning").length;
  const pad = (n: number) => String(n).padStart(2, "0");

  return (
    <div>
      <div className="section" style={{ marginTop: 0 }}>
        <div className="section-head">
          <div>
            <div className="label">ANOMALY DETECTION · MONITORED PROGRAMS</div>
            <h2>Instruction rates exceeding baseline</h2>
          </div>
          <div
            className="shimmer-strip"
            style={{
              padding: "8px 14px",
              border: "1px solid var(--bg-border)",
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              color: "var(--text-secondary)",
              letterSpacing: "0.1em",
              textTransform: "uppercase",
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            <span className="dot static" style={{ background: "var(--safe)" }} />
            auto-refresh 60s · updated {60 - countdown}s ago
          </div>
        </div>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--text-secondary)",
            lineHeight: 1.6,
            maxWidth: 720,
            marginBottom: 22,
          }}
        >
          Watching instructions across your{" "}
          <span style={{ color: "var(--gold)" }}>
            {monitoredPrograms.length} monitored programs
          </span>
          . obsrv compares each instruction&apos;s current call rate against its
          successful baseline and flags anything running hot — exactly where an
          exploit shows up first.
        </p>

        {/* summary cards */}
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: 16,
            marginBottom: 28,
          }}
        >
          <StatCard
            label="ACTIVE SPIKES"
            value={String(spikes.length)}
            accent="warning"
            sub={`${critCount} critical · ${warnCount} warning`}
          />
          <StatCard
            label="ON WATCHED PROGRAMS"
            value={String(monitoredPrograms.length)}
            accent="critical"
            sub="from your monitors"
          />
          <StatCard
            label="NEXT REFRESH"
            value={`${pad(0)}:${pad(countdown)}`}
            sub="auto"
          />
        </div>

        {loading && spikes.length === 0 ? (
          <div
            className="panel"
            style={{
              padding: "60px 32px",
              textAlign: "center",
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--text-tertiary)",
              letterSpacing: "0.1em",
            }}
          >
            LOADING SPIKES<span className="blink">_</span>
          </div>
        ) : monitoredPrograms.length === 0 ? (
          <div
            className="panel"
            style={{
              padding: "60px 32px",
              textAlign: "center",
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--text-tertiary)",
              letterSpacing: "0.1em",
            }}
          >
            NO PROGRAMS MONITORED YET · ADD PROGRAMS IN MONITOR →
          </div>
        ) : spikes.length === 0 ? (
          <div
            className="panel"
            style={{
              padding: "60px 32px",
              textAlign: "center",
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--text-tertiary)",
              letterSpacing: "0.1em",
            }}
          >
            NO INSTRUCTION ACTIVITY YET FOR YOUR MONITORED PROGRAMS
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
            {spikes.map((s, i) => {
              const maxVal = Math.max(s.today, s.baseline) * 1.1 || 1;
              const basePct = (s.baseline / maxVal) * 100;
              const todayPct = (s.today / maxVal) * 100;
              const c =
                s.level === "critical"
                  ? "var(--critical)"
                  : s.level === "warning"
                    ? "var(--warning)"
                    : "var(--info)";
              const progShort =
                s.programId.length > 12
                  ? `${s.programId.slice(0, 4)}…${s.programId.slice(-4)}`
                  : s.programId;
              return (
                <div key={i} className="panel" style={{ padding: "24px 28px" }}>
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "240px 1fr 120px",
                      gap: 32,
                      alignItems: "center",
                    }}
                  >
                    <div>
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 10,
                          marginBottom: 6,
                        }}
                      >
                        <span
                          style={{
                            fontFamily: "var(--font-mono)",
                            fontSize: 14,
                            color: "var(--text-primary)",
                          }}
                        >
                          {s.ix}
                        </span>
                        <span
                          style={{
                            fontFamily: "var(--font-mono)",
                            fontSize: 9,
                            letterSpacing: "0.12em",
                            color: "var(--gold)",
                            border: "1px solid var(--gold-dim)",
                            padding: "2px 6px",
                          }}
                        >
                          WATCHED
                        </span>
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--font-sans)",
                          fontSize: 13,
                          color: "var(--text-secondary)",
                        }}
                      >
                        {s.program === s.programId ? progShort : `${s.program} · ${progShort}`}
                      </div>
                    </div>
                    <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
                      {(
                        [
                          ["BASELINE", basePct, s.baseline, "var(--text-tertiary)"] as const,
                          ["TODAY", todayPct, s.today, c] as const,
                        ]
                      ).map(([lbl, pct, val, col], k) => (
                        <div
                          key={k}
                          style={{ display: "flex", alignItems: "center", gap: 14 }}
                        >
                          <span
                            className="label"
                            style={{
                              width: 72,
                              color: k === 1 ? c : "var(--text-tertiary)",
                            }}
                          >
                            {lbl}
                          </span>
                          <div
                            style={{
                              flex: 1,
                              height: 9,
                              background: "var(--bg-elevated)",
                              position: "relative",
                            }}
                          >
                            <div
                              style={{
                                position: "absolute",
                                inset: 0,
                                width: pct + "%",
                                background: col,
                                transition: "width 900ms cubic-bezier(.2,.8,.2,1)",
                              }}
                            />
                          </div>
                          <span
                            className="data"
                            style={{
                              fontSize: 12,
                              color: col,
                              minWidth: 84,
                              textAlign: "right",
                            }}
                          >
                            {val.toLocaleString()}
                          </span>
                        </div>
                      ))}
                    </div>
                    <div style={{ textAlign: "right" }}>
                      <div
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 28,
                          color: c,
                          lineHeight: 1,
                          fontVariantNumeric: "tabular-nums",
                        }}
                      >
                        {s.factor.toFixed(1)}×
                      </div>
                      <div className="label" style={{ marginTop: 8, color: c }}>
                        ABOVE BASELINE
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

// ── MONITORED TAB ────────────────────────────────────────────────────
function MonitoredTab() {
  const [programs, setPrograms] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const { showError } = useAlerts();

  async function fetchMonitored() {
    try {
      setLoading(true);
      const list = await api.getMonitorList();
      const monitoredProgs = list?.programs ?? [];

      // Fetch stats for each monitored program
      const withStats = await Promise.all(
        monitoredProgs.map(async (prog: any) => {
          try {
            const stats: any = await api.getProgramAnalytics(
              prog.program_id || prog.address,
            );
            return {
              ...prog,
              total_calls: stats?.summary?.total_calls ?? 0,
              success_count: stats?.summary?.success_count ?? 0,
              failed_calls: stats?.summary?.failed_calls ?? 0,
              success_rate: stats?.summary?.success_rate ?? 0,
            };
          } catch {
            return {
              ...prog,
              total_calls: 0,
              success_count: 0,
              failed_calls: 0,
              success_rate: 0,
            };
          }
        }),
      );
      setPrograms(withStats);
    } catch (err) {
      showError("Failed to fetch monitored programs");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchMonitored();
    const interval = setInterval(fetchMonitored, 30000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="panel" style={{ padding: 40, textAlign: "center" }}>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            color: "var(--text-tertiary)",
          }}
        >
          Loading<span className="blink">_</span>
        </span>
      </div>
    );
  }

  if (programs.length === 0) {
    return (
      <div
        className="panel"
        style={{
          padding: "60px 32px",
          textAlign: "center",
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          color: "var(--text-tertiary)",
          letterSpacing: "0.1em",
        }}
      >
        NO PROGRAMS MONITORED YET · ADD PROGRAMS IN MONITOR →
      </div>
    );
  }

  return (
    <div>
      <div className="section" style={{ marginTop: 0 }}>
        <div className="section-head">
          <div>
            <div className="label">{programs.length} PROGRAMS MONITORED</div>
            <h2>Monitored Programs Activity</h2>
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
          {programs.map((prog: any, i: number) => {
            const total = prog.total_calls ?? 0;
            const success = prog.success_count ?? 0;
            const failed = prog.failed_calls ?? 0;
            const failRate = total > 0 ? (failed / total) * 100 : 0;

            return (
              <div
                key={i}
                className="panel"
                style={{
                  marginBottom: 2,
                  borderColor: failRate > 10 ? "var(--warning)" : undefined,
                }}
              >
                <div
                  style={{
                    display: "grid",
                    gridTemplateColumns: "300px 1fr 200px",
                    gap: 24,
                    padding: "20px 24px",
                    alignItems: "center",
                  }}
                >
                  <div>
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 10,
                        marginBottom: 4,
                      }}
                    >
                      <ProgramPill name={prog.name || "Unknown"} />
                      <span
                        className="label"
                        style={{ color: "var(--safe)", fontSize: 10 }}
                      >
                        WATCHED
                      </span>
                    </div>
                    <div
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--text-tertiary)",
                      }}
                    >
                      {prog.program_id || prog.address}
                    </div>
                  </div>

                  <SplitBar success={success} failed={failed} total={total} />

                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "1fr 1fr 1fr",
                      gap: 16,
                      textAlign: "center",
                    }}
                  >
                    <div>
                      <div
                        className="label"
                        style={{ fontSize: 10, marginBottom: 4 }}
                      >
                        TOTAL
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 14,
                          color: "var(--text-primary)",
                        }}
                      >
                        {total.toLocaleString()}
                      </div>
                    </div>
                    <div>
                      <div
                        className="label"
                        style={{
                          fontSize: 10,
                          marginBottom: 4,
                          color: "var(--safe)",
                        }}
                      >
                        SUCCESS
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 14,
                          color: "var(--safe)",
                        }}
                      >
                        {success.toLocaleString()}
                      </div>
                    </div>
                    <div>
                      <div
                        className="label"
                        style={{
                          fontSize: 10,
                          marginBottom: 4,
                          color: "var(--critical)",
                        }}
                      >
                        FAILED
                      </div>
                      <div
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 14,
                          color: "var(--critical)",
                        }}
                      >
                        {failed.toLocaleString()}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

// ── MAIN ───────────────────────────────────────────────────────────
export function PageAnalytics() {
  const [activeTab, setActiveTab] = useState<"wallet" | "program" | "spikes">(
    "wallet",
  );

  return (
    <>
      <div className="page-head">
        <div className="page-head-row">
          <div>
            <div className="label">ANALYTICS</div>
            <h1>See patterns no single transaction reveals</h1>
            <p>
              Aggregate behavior across wallets, programs, and the whole network
              — which programs a wallet quietly added, where a contract burns
              CU, and what&apos;s suddenly running hot.
            </p>
          </div>
        </div>
      </div>

      <div className="tabs">
        {(
          [
            ["wallet", "WALLET"],
            ["program", "PROGRAM"],
            ["spikes", "SPIKES"],
          ] as const
        ).map(([k, l]) => (
          <div
            key={k}
            className={`tab ${activeTab === k ? "active" : ""}`}
            onClick={() => setActiveTab(k as any)}
          >
            <span>{l}</span>
          </div>
        ))}
      </div>

      <div className="page-body">
        {activeTab === "wallet" && <WalletTab />}
        {activeTab === "program" && <ProgramTab />}
        {activeTab === "spikes" && <SpikesTab spikeCount={0} />}
      </div>
    </>
  );
}
