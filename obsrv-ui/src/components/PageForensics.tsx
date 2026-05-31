"use client";

import React, { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { useAlerts } from "./ErrorAlert";
import {
  AddressDisplay,
  RiskBadge,
  ProgramPill,
  StatCard,
  SolAmount,
  LiveBar,
  PulseDot,
} from "@/lib/components";

function Section({
  kicker,
  title,
  right,
  children,
}: {
  kicker?: string;
  title: string;
  right?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="section">
      <div className="section-head">
        <div>
          {kicker && <div className="label">{kicker}</div>}
          <h2>{title}</h2>
        </div>
        {right && (
          <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
            {right}
          </div>
        )}
      </div>
      {children}
    </div>
  );
}

// Strip per-instruction detail, "Risk score: …", and "Fee payer: …" so the
// narration paragraph stays a clean one-paragraph summary.
function cleanSummary(s?: string): string {
  if (!s) return "";
  let out = s;
  const ixIdx = out.search(/Instruction\s+\d+\s*:/i);
  if (ixIdx > 0) out = out.slice(0, ixIdx);
  const riskIdx = out.search(/Risk\s+score\s*:/i);
  if (riskIdx > 0) out = out.slice(0, riskIdx);
  const feeIdx = out.search(/Fee\s+payer\s*:/i);
  if (feeIdx > 0) out = out.slice(0, feeIdx);
  return out.replace(/^[^\w(⚠ℹ✓✗]+/u, "").replace(/\s+/g, " ").trim();
}

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
    n.includes("kamino") ||
    n.includes("drift") ||
    n.includes("marinade") ||
    n.includes("openbook")
  )
    return "serum";
  return "unknown";
}

export function PageForensics() {
  const [sig, setSig] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);
  const [logsOpen, setLogsOpen] = useState(true);
  const { showError } = useAlerts();

  async function handleFetch() {
    if (!sig.trim()) return;
    setLoading(true);
    setResult(null);
    try {
      const data = await api.forensics(sig.trim());
      setResult(data);
    } catch (err) {
      const message =
        err instanceof ApiError ? err.message : "Failed to fetch transaction";
      const detail =
        err instanceof ApiError && err.data?.timeout
          ? "Please ensure the obsrv API server is running on localhost:3001"
          : undefined;
      showError(message, detail);
    } finally {
      setLoading(false);
    }
  }

  const cuUsed = result?.cu_consumed ?? 0;
  const cuBudget =
    (result?.raw?.tx?.analysis?.cu_limit ?? cuUsed * 1.2) || 200000;
  const cuPct = cuBudget > 0 ? (cuUsed / cuBudget) * 100 : 0;
  const feeSol =
    result?.fee_sol ?? (result?.fee_lamports ? result.fee_lamports / 1e9 : 0);
  const status: string = result?.execution_status ?? "unknown";
  const isSuccess = status === "success";
  const riskScore = Math.round(result?.risk_score ?? 0);
  const slot = result?.raw?.tx?.meta?.slot ?? result?.raw?.tx?.slot;
  const summary: string = cleanSummary(
    result?.summary ?? result?.raw?.tx?.analysis?.summary,
  );

  // unique programs
  const programs: { name: string; type: ReturnType<typeof programType> }[] = [];
  if (result?.raw?.tx?.instructions) {
    const seen = new Set<string>();
    for (const ix of result.raw.tx.instructions) {
      const name = ix.program || ix.program_name || "Unknown";
      if (!seen.has(name)) {
        seen.add(name);
        programs.push({ name, type: programType(name) });
      }
    }
  }

  // Flatten the backend's `balances.changes` into one row per (account, asset).
  // Field names match `obsrv-core::types::{SolChange,TokenChange}` exactly.
  const balances: {
    addr: string;
    label?: string;
    before: number;
    after: number;
    change: number;
    sym: string;
  }[] = [];
  if (result?.raw?.tx?.balances?.changes) {
    for (const c of result.raw.tx.balances.changes) {
      if (c.sol) {
        balances.push({
          addr: c.address,
          label: c.label,
          before: Number(c.sol.pre_sol ?? 0),
          after: Number(c.sol.post_sol ?? 0),
          change: Number(c.sol.change_sol ?? 0),
          sym: "SOL",
        });
      }
      for (const tok of c.tokens ?? []) {
        balances.push({
          addr: c.address,
          label: tok.owner ? `mint ${tok.mint.slice(0, 4)}…` : undefined,
          before: Number(tok.pre_ui ?? 0),
          after: Number(tok.post_ui ?? 0),
          change: Number(tok.change_ui ?? 0),
          sym: tok.mint ? `${tok.mint.slice(0, 4)}…${tok.mint.slice(-4)}` : "TOKEN",
        });
      }
    }
  }

  const rawLogs: string[] = result?.logs ?? [];
  const logs = rawLogs.map((line) => {
    const isErr = /failed|error/i.test(line);
    const isOk = /success|invoke/i.test(line);
    const type: "ok" | "err" | "log" = isErr ? "err" : isOk ? "ok" : "log";
    const indent = ((line.match(/^\s+/) ?? [""])[0].length / 2) | 0;
    return { type, text: line.trim(), indent };
  });

  return (
    <>
      <div className="page-head">
        <div className="page-head-row">
          <div>
            <div className="label">FORENSICS</div>
            <h1>Reconstruct any confirmed transaction</h1>
            <p>
              Fetch a signature and obsrv pulls the full execution trace —
              compute usage, balance deltas, inner instructions, and a
              plain-English narration of what really happened.
            </p>
          </div>
        </div>
      </div>

      <div className="page-body" style={{ maxWidth: 1200 }}>
        {/* Signature input */}
        <div className="panel">
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "1fr auto",
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
              <span className="label">SIG</span>
              <input
                value={sig}
                onChange={(e) => setSig(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleFetch()}
                placeholder="paste signature…"
                disabled={loading}
                style={{
                  flex: 1,
                  fontFamily: "var(--font-mono)",
                  fontSize: 13,
                  color: "var(--text-primary)",
                }}
              />
            </div>
            <button
              className="btn btn-primary"
              style={{ borderRadius: 0, border: 0, padding: "0 28px" }}
              onClick={handleFetch}
              disabled={loading || !sig.trim()}
            >
              {loading ? (
                <>
                  Fetching<span className="blink">_</span>
                </>
              ) : (
                "Fetch & analyze"
              )}
            </button>
          </div>
        </div>

        {result && (
          <>
            {/* KPI strip */}
            <Section kicker="OVERVIEW" title="At a glance">
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "repeat(4, 1fr)",
                  gap: 16,
                }}
              >
                <StatCard
                  label="EXECUTION"
                  value={
                    <span
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: 12,
                      }}
                    >
                      <PulseDot color={isSuccess ? "safe" : "critical"} />
                      {status.toUpperCase()}
                    </span>
                  }
                  accent={isSuccess ? "safe" : "critical"}
                  sub={
                    slot ? `slot ${Number(slot).toLocaleString()}` : undefined
                  }
                />
                <StatCard
                  label="COMPUTE UNITS"
                  value={`${(cuUsed / 1000).toFixed(1)}K`}
                  sub={`${Math.round(cuPct)}% of ${(cuBudget / 1000).toFixed(0)}K budget`}
                  footer={<LiveBar value={cuUsed} max={cuBudget} />}
                />
                <StatCard
                  label="NETWORK FEE"
                  value={feeSol.toFixed(6)}
                  sub="SOL · no priority fee"
                />
                <StatCard
                  label="RISK SCORE"
                  value={`${riskScore}/10`}
                  accent={
                    riskScore >= 8
                      ? "critical"
                      : riskScore >= 5
                        ? "warning"
                        : "safe"
                  }
                  sub={
                    result.failure_reason
                      ? String(result.failure_reason).slice(0, 40)
                      : riskScore >= 5
                        ? "review flagged signals"
                        : "no suspicious patterns"
                  }
                />
              </div>
            </Section>

            {/* Narrative — always shown */}
            <Section kicker="WHAT HAPPENED" title="Plain-English narration">
              <div className="panel" style={{ padding: "32px 36px" }}>
                <p
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 16,
                    lineHeight: 1.65,
                    color: summary ? "var(--text-primary)" : "var(--text-tertiary)",
                    maxWidth: 780,
                  }}
                >
                  {summary ||
                    "No narration available for this transaction yet."}
                </p>
                <div
                  style={{
                    marginTop: 24,
                    paddingTop: 24,
                    borderTop: "1px solid var(--bg-border)",
                  }}
                >
                  <div className="label" style={{ marginBottom: 12 }}>
                    PROGRAMS INVOKED · {programs.length}
                  </div>
                  {programs.length > 0 ? (
                    <div
                      style={{ display: "flex", flexWrap: "wrap", gap: 8 }}
                    >
                      {programs.map((p, i) => (
                        <ProgramPill key={i} name={p.name} type={p.type} />
                      ))}
                    </div>
                  ) : (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--text-tertiary)",
                        letterSpacing: "0.08em",
                      }}
                    >
                      —
                    </span>
                  )}
                </div>
              </div>
            </Section>

            {/* Balance changes — always shown */}
            <Section kicker="STATE CHANGES" title="Balances affected">
              <div className="panel">
                {balances.length > 0 ? (
                  <table className="tbl">
                    <thead>
                      <tr>
                        <th>Account</th>
                        <th>Label</th>
                        <th className="num">Before</th>
                        <th className="num">After</th>
                        <th className="num">Change</th>
                      </tr>
                    </thead>
                    <tbody>
                      {balances.map((b, i) => (
                        <tr key={i} className="row-hover">
                          <td>
                            <AddressDisplay address={b.addr} />
                          </td>
                          <td>
                            {b.label ? (
                              <RiskBadge level="neutral" size="sm">
                                {b.label}
                              </RiskBadge>
                            ) : (
                              <span style={{ color: "var(--text-tertiary)" }}>
                                —
                              </span>
                            )}
                          </td>
                          <td className="num">
                            <span style={{ color: "var(--text-secondary)" }}>
                              {b.before.toLocaleString("en-US", {
                                maximumFractionDigits: 6,
                              })}{" "}
                              {b.sym}
                            </span>
                          </td>
                          <td className="num">
                            {b.after.toLocaleString("en-US", {
                              maximumFractionDigits: 6,
                            })}{" "}
                            {b.sym}
                          </td>
                          <td className="num">
                            <SolAmount value={b.change} signed sym={b.sym} />
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <div
                    style={{
                      padding: "48px 32px",
                      textAlign: "center",
                      fontFamily: "var(--font-mono)",
                      fontSize: 12,
                      color: "var(--text-tertiary)",
                      letterSpacing: "0.1em",
                    }}
                  >
                    NO BALANCE CHANGES DETECTED FOR THIS TRANSACTION
                  </div>
                )}
              </div>
            </Section>

            {/* Logs */}
            {logs.length > 0 && (
              <Section
                kicker="RAW OUTPUT"
                title="Program logs"
                right={
                  <button
                    className="btn btn-ghost btn-sm"
                    onClick={() => setLogsOpen(!logsOpen)}
                  >
                    {logsOpen ? "Collapse" : "Expand"} ↓
                  </button>
                }
              >
                {logsOpen && (
                  <div
                    className="panel"
                    style={{
                      background: "#060606",
                      borderColor: "var(--bg-border-strong)",
                      padding: "24px 28px",
                      fontFamily: "var(--font-mono)",
                      fontSize: 12,
                      lineHeight: 1.85,
                      maxHeight: 460,
                      overflowY: "auto",
                    }}
                  >
                    {logs.map((line, i) => {
                      const color =
                        line.type === "ok"
                          ? "var(--term-green)"
                          : line.type === "err"
                            ? "var(--critical)"
                            : "#9aa39a";
                      return (
                        <div
                          key={i}
                          style={{
                            color,
                            whiteSpace: "pre",
                            paddingLeft: line.indent * 16,
                          }}
                        >
                          <span
                            style={{
                              color: "var(--text-tertiary)",
                              marginRight: 8,
                            }}
                          >
                            ›
                          </span>
                          {line.text}
                        </div>
                      );
                    })}
                    <div style={{ color: "var(--term-green)", marginTop: 6 }}>
                      › <span className="blink">_</span>
                    </div>
                  </div>
                )}
              </Section>
            )}
          </>
        )}
      </div>
    </>
  );
}
