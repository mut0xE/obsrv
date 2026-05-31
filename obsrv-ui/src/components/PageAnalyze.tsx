"use client";

import React, { useState, useRef, useEffect } from "react";
import { api, ApiError } from "@/lib/api";
import { useAlerts } from "./ErrorAlert";
import {
  RiskBadge,
  AddressDisplay,
  ProgramPill,
  PulseDot,
} from "@/lib/components";

const SAMPLE_TX_BYTES =
  "01a7d3c8b2f1e9a04d7c5b1e2f3a8d6c9b4e1f5a2d8c3b7e0a1f4d9c2b5e8a3f7d0c1b4e9a6f2d5c8b3e0a7d1c4b9e2f5a8d6c3b0e7f1a4d9c2b5e8a3f7d0c1b4e9a040100050709a3f1b2e4d6c8…";

// ── helpers ────────────────────────────────────────────────────────
function Section({
  kicker,
  title,
  right,
  children,
  divRef,
}: {
  kicker?: string;
  title: string;
  right?: React.ReactNode;
  children: React.ReactNode;
  divRef?: React.RefObject<HTMLDivElement | null>;
}) {
  return (
    <div className="section" ref={divRef}>
      <div className="section-head">
        <div>
          {kicker && <div className="label">{kicker}</div>}
          <h2>{title}</h2>
        </div>
        {right && <div style={{ display: "flex", gap: 8, alignItems: "center" }}>{right}</div>}
      </div>
      {children}
    </div>
  );
}

function programType(name: string): "system" | "token" | "compute" | "serum" | "unknown" {
  const n = (name || "").toLowerCase();
  if (n.includes("compute")) return "compute";
  if (n.includes("token") || n.includes("spl")) return "token";
  if (n.includes("system") || n.includes("1111")) return "system";
  if (n.includes("jupiter") || n.includes("serum") || n.includes("mango") || n.includes("drift") || n.includes("kamino")) return "serum";
  return "unknown";
}

function looksLikeSignature(s: string): boolean {
  const t = s.trim();
  // Base58 signatures are typically 87–88 chars, no spaces, no hex prefix
  return t.length >= 80 && t.length <= 100 && /^[1-9A-HJ-NP-Za-km-z]+$/.test(t);
}

// ── CountUp ─────────────────────────────────────────────────────────
function CountUp({ target, duration = 900 }: { target: number; duration?: number }) {
  const [val, setVal] = useState(0);
  useEffect(() => {
    let raf = 0;
    const start = performance.now();
    const tick = (now: number) => {
      const t = Math.min(1, (now - start) / duration);
      const ease = 1 - Math.pow(1 - t, 3);
      setVal(target * ease);
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [target, duration]);
  return <>{Math.round(val)}</>;
}

// ── INPUT PANEL ─────────────────────────────────────────────────────
function InputPanel({
  value,
  setValue,
  onAnalyze,
  onSimulate,
  isAnalyzing,
}: {
  value: string;
  setValue: (s: string) => void;
  onAnalyze: () => void;
  onSimulate: () => void;
  isAnalyzing: boolean;
}) {
  return (
    <div className="panel" style={{ position: "relative" }}>
      <div style={{ padding: "22px 30px 16px", display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <div className="label" style={{ marginBottom: 5 }}>STEP 01</div>
          <div className="panel-title">Paste signature or transaction bytes</div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn-ghost btn-sm" onClick={() => setValue(SAMPLE_TX_BYTES)}>Load sample</button>
          <button className="btn btn-ghost btn-sm" onClick={() => setValue("")}>Clear</button>
        </div>
      </div>
      <div style={{ position: "relative", borderTop: "1px solid var(--bg-border)" }}>
        {isAnalyzing ? <div className="scan-line" /> : null}
        <textarea
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="paste a signature (base58) or raw transaction bytes…"
          spellCheck={false}
          disabled={isAnalyzing}
          style={{
            width: "100%", minHeight: 150, background: "var(--bg-void)", color: "var(--text-primary)",
            fontFamily: "var(--font-mono)", fontSize: 14, lineHeight: 1.8,
            padding: "22px 30px 34px", border: "none", resize: "vertical", wordBreak: "break-all",
          }}
        />
        <div style={{ position: "absolute", bottom: 12, right: 30, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-tertiary)", letterSpacing: "0.06em" }}>
          {value.length.toLocaleString()} chars
        </div>
      </div>
      <div style={{ display: "flex", gap: 12, padding: "20px 30px", borderTop: "1px solid var(--bg-border)", alignItems: "center" }}>
        <button className="btn btn-primary" onClick={onAnalyze} disabled={isAnalyzing || !value.trim()} style={{ padding: "14px 24px" }}>
          {isAnalyzing ? <>Analyzing<span className="blink">_</span></> : <>▸ Analyze transaction</>}
        </button>
        <button className="btn btn-info" onClick={onSimulate} disabled={isAnalyzing || !value.trim()} style={{ padding: "14px 24px" }}>
          ⟳ Simulate only
        </button>
        <div style={{ marginLeft: "auto", display: "flex", gap: 14, alignItems: "center" }}>
          <span className="label">Cluster</span>
          <select style={{ background: "var(--bg-surface)", border: "1px solid var(--bg-border-strong)", color: "var(--text-primary)", fontFamily: "var(--font-mono)", fontSize: 13, padding: "9px 13px", letterSpacing: "0.04em" }}>
            <option>mainnet-beta</option><option>devnet</option><option>testnet</option>
          </select>
        </div>
      </div>
    </div>
  );
}

// ── Risk Meter ─────────────────────────────────────────────────────
function RiskMeter({ score }: { score: number }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4, height: "100%", justifyContent: "center" }}>
      {Array.from({ length: 10 }).map((_, i) => {
        const seg = 10 - i;
        const on = seg <= score;
        const segColor = seg >= 8 ? "var(--critical)" : seg >= 5 ? "var(--warning)" : "var(--safe)";
        return (
          <div key={i} style={{ display: "flex", alignItems: "center", gap: 12 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: on ? segColor : "var(--text-quat)", width: 18, textAlign: "right", fontVariantNumeric: "tabular-nums" }}>{seg}</span>
            <div style={{
              width: 56, height: 10,
              background: on ? segColor : "var(--bg-elevated)",
              opacity: on ? 0.5 + seg / 20 : 1,
              boxShadow: seg === score ? `0 0 12px ${segColor}` : "none",
              transition: "all 400ms",
            }} />
          </div>
        );
      })}
    </div>
  );
}

// ── VERDICT HERO ───────────────────────────────────────────────────
function VerdictHero({ result, onJump }: { result: any; onJump: () => void }) {
  const score = Math.round(result.risk_score ?? 0);
  const level: "critical" | "warning" | "safe" = score >= 8 ? "critical" : score >= 5 ? "warning" : "safe";
  const accent = `var(--${level})`;
  const dim = `var(--${level}-dim)`;
  const verdict = score >= 8 ? "DO NOT SIGN" : score >= 5 ? "REVIEW CAREFULLY" : "LOOKS SAFE";

  const flags: any[] = result.raw?.tx?.analysis?.flags ?? result.flags ?? [];
  const norm = (f: any) =>
    typeof f === "string"
      ? f.toLowerCase().includes("critical") ? "critical" : f.toLowerCase().includes("warn") ? "warning" : "info"
      : f?.severity ?? "info";
  const critCount = flags.filter((f) => norm(f) === "critical").length;
  const warnCount = flags.filter((f) => norm(f) === "warning").length;

  const headline = result.headline ?? result.summary ?? result.raw?.tx?.analysis?.summary ?? "Analysis complete.";
  const reasoning = result.reasoning ?? result.raw?.tx?.analysis?.reasoning ?? "";

  return (
    <div
      className={`panel ${level === "critical" ? "crit-glow" : ""}`}
      style={{
        borderColor: accent, borderLeftWidth: 0,
        background: `linear-gradient(115deg, ${dim} 0%, var(--bg-deep) 62%)`,
        position: "relative", overflow: "hidden",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "22px 36px", borderBottom: "1px solid var(--bg-border)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <PulseDot color={level} />
          <span className="label" style={{ color: accent }}>VERDICT</span>
        </div>
        <span className="label">RISK ASSESSMENT · MAINNET-BETA</span>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "260px 1fr 220px", alignItems: "stretch" }}>
        {/* zone 1 — score */}
        <div style={{ padding: "40px 36px", display: "flex", flexDirection: "column", justifyContent: "center", borderRight: "1px solid var(--bg-border)" }}>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 120, lineHeight: 0.85, color: accent, fontWeight: 600, letterSpacing: "-0.05em", fontVariantNumeric: "tabular-nums" }}>
            <CountUp target={score} />
          </div>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--text-secondary)", letterSpacing: "0.18em", textTransform: "uppercase", marginTop: 14 }}>
            of 10 · risk score
          </div>
          <div style={{ display: "flex", gap: 16, marginTop: 22 }}>
            {critCount > 0 && (
              <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <span className="dot static" style={{ background: "var(--critical)" }} />
                <span className="label-strong" style={{ color: "var(--text-secondary)" }}>{critCount} crit</span>
              </span>
            )}
            {warnCount > 0 && (
              <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <span className="dot static" style={{ background: "var(--warning)" }} />
                <span className="label-strong" style={{ color: "var(--text-secondary)" }}>{warnCount} warn</span>
              </span>
            )}
          </div>
        </div>

        {/* zone 2 — verdict copy */}
        <div style={{ padding: "40px 40px", display: "flex", flexDirection: "column", justifyContent: "center" }}>
          <div style={{ fontFamily: "var(--font-sans)", fontWeight: 700, fontSize: 46, color: accent, letterSpacing: "-0.025em", lineHeight: 1.02 }}>{verdict}</div>
          <div style={{ fontFamily: "var(--font-sans)", fontWeight: 500, fontSize: 21, color: "var(--text-primary)", marginTop: 18, letterSpacing: "-0.01em", lineHeight: 1.4, maxWidth: 560 }}>{headline}</div>
          {reasoning && (
            <div style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--text-secondary)", marginTop: 14, lineHeight: 1.6, maxWidth: 560 }}>{reasoning}</div>
          )}
          <div style={{ display: "flex", gap: 10, marginTop: 30 }}>
            {level === "critical" && (
              <button className="btn btn-critical" style={{ padding: "14px 24px" }}>Reject &amp; quarantine</button>
            )}
            <button className="btn btn-ghost" style={{ padding: "14px 24px", border: "1px solid var(--bg-border-strong)" }} onClick={onJump}>Jump to evidence ↓</button>
          </div>
        </div>

        {/* zone 3 — risk meter */}
        <div style={{ padding: "36px 32px", display: "flex", flexDirection: "column", justifyContent: "center", borderLeft: "1px solid var(--bg-border)", background: "rgba(0,0,0,0.15)" }}>
          <div className="label" style={{ marginBottom: 18, textAlign: "center" }}>RISK SCALE</div>
          <RiskMeter score={score} />
        </div>
      </div>
    </div>
  );
}

// ── FINDINGS ───────────────────────────────────────────────────────
function FindingsList({ flags }: { flags: any[] }) {
  if (!flags || flags.length === 0) return null;
  const normalized = flags.map((f: any) =>
    typeof f === "string"
      ? {
          severity: f.toLowerCase().includes("critical") ? "critical" : f.toLowerCase().includes("warn") ? "warning" : "info",
          title: f,
          detail: "",
          weight: undefined,
        }
      : { severity: f.severity ?? "info", title: f.title ?? f.name ?? "Finding", detail: f.detail ?? f.description ?? "", weight: f.weight }
  );
  const sevOrder: Record<string, number> = { critical: 0, warning: 1, info: 2, safe: 3 };
  normalized.sort((a: any, b: any) => (sevOrder[a.severity] ?? 9) - (sevOrder[b.severity] ?? 9));

  return (
    <div className="panel">
      <div className="panel-head">
        <span className="panel-title">{normalized.length} findings</span>
        <span className="panel-sub">— sorted by severity</span>
      </div>
      <div>
        {normalized.map((f: any, i: number) => (
          <div key={i} style={{
            display: "grid", gridTemplateColumns: "3px 1fr auto", gap: 22,
            padding: "24px 30px",
            borderBottom: i < normalized.length - 1 ? "1px solid var(--bg-border)" : "none",
            alignItems: "center",
          }}>
            <div className={`sev-bar ${f.severity}`} />
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 7 }}>
                <RiskBadge level={f.severity} size="sm">{f.severity}</RiskBadge>
                <span style={{ fontFamily: "var(--font-sans)", fontWeight: 600, fontSize: 16, color: "var(--text-primary)", letterSpacing: "-0.005em" }}>{f.title}</span>
              </div>
              {f.detail && (
                <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--text-secondary)", lineHeight: 1.55, maxWidth: 660 }}>{f.detail}</div>
              )}
            </div>
            {f.weight != null && (
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 15, color: `var(--${f.severity})`, letterSpacing: "0.05em", fontVariantNumeric: "tabular-nums" }}>
                {String(f.weight).startsWith("+") || String(f.weight).startsWith("-") ? f.weight : `+${f.weight}`}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// ── NONCE PANEL ────────────────────────────────────────────────────
function NoncePanel({ result }: { result: any }) {
  if (!result.is_durable_nonce) return null;
  const nonce = result.nonce ?? result.raw?.tx?.meta?.nonce ?? {};
  return (
    <div className="panel" style={{ borderColor: "var(--critical)", background: "linear-gradient(160deg, var(--critical-dim) 0%, var(--bg-deep) 70%)", padding: "30px 36px" }}>
      <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: 26, alignItems: "center" }}>
        <div style={{ width: 58, height: 58, border: "1px solid var(--critical)", color: "var(--critical)", display: "grid", placeItems: "center", fontFamily: "var(--font-mono)", fontSize: 28, background: "rgba(255,69,58,0.06)" }}>!</div>
        <div>
          <div style={{ fontFamily: "var(--font-sans)", fontWeight: 600, fontSize: 19, color: "var(--critical)" }}>Durable nonce detected — this transaction never expires</div>
          <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--text-secondary)", marginTop: 9, maxWidth: 740, lineHeight: 1.55 }}>
            Unlike normal transactions (which expire after ~60s), durable-nonce txs remain valid until explicitly cancelled. A malicious actor could replay this one weeks or months from now.
          </div>
          {(nonce.account || nonce.authority) && (
            <div style={{ display: "grid", gridTemplateColumns: "130px 1fr", gap: "10px 22px", marginTop: 20 }}>
              {nonce.account && (<>
                <span className="label">Nonce account</span>
                <AddressDisplay address={nonce.account} />
              </>)}
              {nonce.authority && (<>
                <span className="label">Authority</span>
                <AddressDisplay address={nonce.authority} />
              </>)}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ── META STRIP ─────────────────────────────────────────────────────
function MetaStrip({ result }: { result: any }) {
  const tx = result.raw?.tx ?? {};
  const signerCount =
    tx.meta?.signer_count ??
    (Array.isArray(tx.instructions)
      ? tx.instructions.flatMap((ix: any) => ix.accounts ?? []).filter((a: any) => typeof a === "object" && a.is_signer).length
      : undefined) ?? "—";

  const items = [
    { label: "INSTRUCTIONS", value: tx.instructions?.length ?? "—" },
    { label: "ACCOUNTS", value: tx.meta?.account_count ?? tx.balances?.changes?.length ?? "—" },
    { label: "SIGNERS", value: signerCount },
    { label: "COMPUTE", value: result.cu_consumed ? `${(result.cu_consumed / 1000).toFixed(0)}K CU` : "—" },
    { label: "FEE", value: result.fee_sol != null ? `${Number(result.fee_sol).toFixed(6)} SOL` : "—" },
  ];

  return (
    <div className="panel" style={{ display: "grid", gridTemplateColumns: `repeat(${items.length + 1}, 1fr)` }}>
      {items.map((it, i) => (
        <div key={i} style={{ padding: "22px 26px", borderRight: "1px solid var(--bg-border)" }}>
          <div className="label" style={{ marginBottom: 12 }}>{it.label}</div>
          <div style={{ fontFamily: "var(--font-data)", fontSize: 19, color: "var(--text-primary)", fontVariantNumeric: "tabular-nums" }}>{it.value}</div>
        </div>
      ))}
      <div style={{ padding: "22px 26px" }}>
        <div className="label" style={{ marginBottom: 12 }}>FEE PAYER</div>
        {result.fee_payer ? <AddressDisplay address={result.fee_payer} /> : <span style={{ color: "var(--text-tertiary)" }}>—</span>}
      </div>
    </div>
  );
}

// ── INSTRUCTION LIST ───────────────────────────────────────────────
function roleColor(role: string) {
  return role === "signer" ? "var(--gold)" : role === "writable" ? "var(--warning)" : role === "delegate" ? "var(--critical)" : "var(--text-tertiary)";
}

function InstructionRow({ ix, idx, expanded, onToggle, last }: { ix: any; idx: number; expanded: boolean; onToggle: () => void; last: boolean }) {
  const sev = ix.severity ?? ix.risk_level ?? "info";
  const accounts: any[] = ix.accounts ?? [];
  const rawArgs = ix.decoded_args ?? ix.args ?? {};
  const argEntries: any[] = Array.isArray(rawArgs)
    ? rawArgs
    : Object.entries(rawArgs).map(([k, v]: [string, any]) => ({
        name: k,
        type: typeof v === "object" && v ? v.type : typeof v,
        value: typeof v === "object" && v ? v.value ?? JSON.stringify(v) : String(v),
        flag: typeof v === "object" && v ? v.flag : undefined,
        note: typeof v === "object" && v ? v.note : undefined,
      }));
  const cpi: any[] = ix.cpi ?? ix.inner_instructions ?? [];
  const action = ix.action ?? ix.type ?? ix.instruction_type ?? "Unknown";
  const short = ix.short ?? ix.summary ?? ix.description ?? "";
  const programName = ix.program ?? ix.program_name ?? "Unknown";
  const programId = ix.program_id ?? ix.programId ?? ix.program ?? "—";

  return (
    <div style={{ borderBottom: last ? "none" : "1px solid var(--bg-border)" }}>
      <div
        onClick={onToggle}
        style={{
          display: "grid", gridTemplateColumns: "3px 46px 1fr 210px 116px 24px", gap: 22,
          padding: "20px 30px 20px 0", alignItems: "center", cursor: "pointer", transition: "background 100ms",
        }}
        onMouseEnter={(e) => ((e.currentTarget as HTMLDivElement).style.background = "var(--bg-surface)")}
        onMouseLeave={(e) => ((e.currentTarget as HTMLDivElement).style.background = "transparent")}
      >
        <div className={`sev-bar ${sev}`} />
        <div style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--text-tertiary)", textAlign: "right" }}>{String(idx).padStart(2, "0")}</div>
        <div>
          <div style={{ fontFamily: "var(--font-sans)", fontWeight: 600, fontSize: 15, color: "var(--text-primary)", marginBottom: 4 }}>{action}</div>
          <div style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--text-secondary)" }}>{short}</div>
        </div>
        <ProgramPill name={programName} type={programType(programName)} />
        <RiskBadge level={sev} size="sm">{sev}</RiskBadge>
        <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-tertiary)", fontSize: 17, transition: "transform 200ms", transform: expanded ? "rotate(90deg)" : "rotate(0deg)" }}>›</span>
      </div>

      {expanded && (
        <div style={{ background: "var(--bg-surface)", borderTop: "1px solid var(--bg-border)", padding: "26px 30px 28px 71px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 24 }}>
            <span className="label" style={{ width: 110 }}>PROGRAM ID</span>
            <AddressDisplay address={programId} full />
          </div>

          <div style={{ display: "grid", gridTemplateColumns: accounts.length ? "1fr 1fr" : "1fr", gap: 32 }}>
            <div>
              <div className="label" style={{ marginBottom: 14 }}>DECODED ARGUMENTS</div>
              {argEntries.length ? (
                <div style={{ border: "1px solid var(--bg-border)" }}>
                  {argEntries.map((a, i) => (
                    <div key={i} style={{
                      display: "grid", gridTemplateColumns: "150px 70px 1fr", gap: 14, alignItems: "baseline",
                      padding: "11px 14px",
                      borderBottom: i < argEntries.length - 1 ? "1px solid var(--bg-border)" : "none",
                      background: a.flag === "critical" ? "var(--critical-dim)" : "transparent",
                    }}>
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: a.flag ? `var(--${a.flag})` : "var(--text-primary)" }}>{a.name}</span>
                      <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--text-tertiary)" }}>{a.type}</span>
                      <span style={{ fontFamily: "var(--font-data)", fontSize: 13, color: a.flag ? `var(--${a.flag})` : "var(--text-secondary)", wordBreak: "break-all" }}>
                        {String(a.value)}
                        {a.note && <span style={{ display: "block", fontFamily: "var(--font-sans)", fontSize: 12, color: `var(--${a.flag})`, marginTop: 3 }}>⚠ {a.note}</span>}
                      </span>
                    </div>
                  ))}
                </div>
              ) : (
                <div style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--text-tertiary)" }}>no arguments</div>
              )}
            </div>

            {accounts.length > 0 && (
              <div>
                <div className="label" style={{ marginBottom: 14 }}>ACCOUNT LIST · {accounts.length}</div>
                <div style={{ border: "1px solid var(--bg-border)" }}>
                  {accounts.map((ac: any, i: number) => {
                    const role = ac.role
                      ?? (ac.is_signer ? "signer" : ac.is_writable ? "writable" : "readonly");
                    const addr = ac.addr ?? ac.address ?? ac.pubkey ?? (typeof ac === "string" ? ac : "");
                    return (
                      <div key={i} style={{
                        display: "grid", gridTemplateColumns: "84px 1fr", gap: 14, alignItems: "center",
                        padding: "11px 14px",
                        borderBottom: i < accounts.length - 1 ? "1px solid var(--bg-border)" : "none",
                        background: ac.flag ? `var(--${ac.flag}-dim)` : "transparent",
                      }}>
                        <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, letterSpacing: "0.1em", textTransform: "uppercase", color: roleColor(role) }}>{role}</span>
                        <div>
                          <AddressDisplay address={addr} />
                          {ac.note && <div style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: ac.flag ? `var(--${ac.flag})` : "var(--text-tertiary)", marginTop: 3 }}>{ac.note}</div>}
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>

          {cpi.length > 0 ? (
            <div style={{ marginTop: 28 }}>
              <div className="label" style={{ marginBottom: 14 }}>CPI CALL TREE</div>
              <div style={{ border: "1px solid var(--bg-border)", background: "#0c0d0f", padding: "14px 18px", fontFamily: "var(--font-mono)", fontSize: 13, lineHeight: 1.9 }}>
                <div style={{ color: "var(--text-secondary)" }}>
                  <span style={{ color: "var(--text-tertiary)" }}>├─</span> {programName}::{action}
                </div>
                {cpi.map((c: any, i: number) => (
                  <div key={i} style={{ color: c.sev ? `var(--${c.sev})` : "var(--text-secondary)", paddingLeft: (c.depth ?? 1) * 22 }}>
                    <span style={{ color: "var(--text-tertiary)" }}>{i === cpi.length - 1 ? "└─" : "├─"}</span> ↳ CPI [{c.depth ?? 1}] {c.program ?? "?"}::{c.action ?? c.type ?? "?"}
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div style={{ marginTop: 24, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-tertiary)" }}>NO INNER INSTRUCTIONS (CPI)</div>
          )}
        </div>
      )}
    </div>
  );
}

function InstructionList({ instructions }: { instructions: any[] }) {
  const [expanded, setExpanded] = useState<Set<number>>(new Set());
  function toggle(i: number) {
    const n = new Set(expanded);
    if (n.has(i)) n.delete(i); else n.add(i);
    setExpanded(n);
  }
  return (
    <div className="panel">
      <div className="panel-head">
        <span className="panel-title">Instruction sequence</span>
        <span className="panel-sub">— {instructions.length} ix · click any row to decode</span>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button className="btn btn-ghost btn-sm" onClick={() => setExpanded(new Set(instructions.map((_, i) => i)))}>Expand all</button>
          <button className="btn btn-ghost btn-sm" onClick={() => setExpanded(new Set())}>Collapse</button>
        </div>
      </div>
      <div>
        {instructions.map((ix: any, i: number) => (
          <InstructionRow key={i} ix={ix} idx={i} expanded={expanded.has(i)} onToggle={() => toggle(i)} last={i === instructions.length - 1} />
        ))}
      </div>
    </div>
  );
}

// ── MAIN ───────────────────────────────────────────────────────────
export function PageAnalyze() {
  const [bytes, setBytes] = useState("");
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [result, setResult] = useState<any>(null);
  const { showError } = useAlerts();
  const evidenceRef = useRef<HTMLDivElement>(null);

  async function runAnalyze() {
    const input = bytes.trim();
    if (!input || isAnalyzing) return;
    setIsAnalyzing(true);
    setResult(null);
    try {
      const data = looksLikeSignature(input)
        ? await api.analyze(input)
        : await api.simulate(input);
      setResult(data);
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : "Failed to analyze";
      const detail = err instanceof ApiError && err.data?.timeout ? "Please ensure the obsrv API server is running on localhost:3001" : undefined;
      showError(msg, detail);
    } finally {
      setIsAnalyzing(false);
    }
  }

  async function runSimulate() {
    const input = bytes.trim();
    if (!input || isAnalyzing) return;
    setIsAnalyzing(true);
    setResult(null);
    try {
      const data = await api.simulate(input);
      setResult(data);
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : "Failed to simulate";
      showError(msg);
    } finally {
      setIsAnalyzing(false);
    }
  }

  function jumpToEvidence() {
    evidenceRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  const flags: any[] = result?.raw?.tx?.analysis?.flags ?? result?.flags ?? [];
  const instructions: any[] = result?.raw?.tx?.instructions ?? result?.instructions ?? [];

  return (
    <>
      <div className="page-head">
        <div className="page-head-row">
          <div>
            <div className="label">ANALYZE</div>
            <h1>Inspect a transaction before you sign it</h1>
            <p>Paste a signature or raw bytes. obsrv decompiles every instruction, simulates against the live cluster, and tells you in plain language exactly what will happen.</p>
          </div>
          <div style={{ display: "flex", gap: 10 }}>
            <button className="btn btn-ghost btn-sm">⌘K · History</button>
            <button className="btn btn-ghost btn-sm">Export report</button>
          </div>
        </div>
      </div>

      <div className="page-body">
        <InputPanel value={bytes} setValue={setBytes} onAnalyze={runAnalyze} onSimulate={runSimulate} isAnalyzing={isAnalyzing} />

        {isAnalyzing ? (
          <div className="section">
            <div className="panel" style={{ padding: "96px 32px", textAlign: "center" }}>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: 13, letterSpacing: "0.3em", color: "var(--info)" }}>DECODING<span className="blink">_</span></div>
              <div style={{ marginTop: 18, fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--text-tertiary)", letterSpacing: "0.15em" }}>parsing header · resolving accounts · matching IDLs · simulating</div>
            </div>
          </div>
        ) : result ? (
          <>
            <Section kicker="STEP 02 — VERDICT" title="What we found" right={<span className="label">REPORT</span>}>
              <VerdictHero result={result} onJump={jumpToEvidence} />
            </Section>

            <div ref={evidenceRef}>
              <Section kicker="EVIDENCE" title={`Why this scored ${Math.round(result.risk_score ?? 0)} / 10`}>
                <FindingsList flags={flags} />
              </Section>
            </div>

            {result.is_durable_nonce && (
              <Section kicker="CRITICAL FLAG" title="Replay risk">
                <NoncePanel result={result} />
              </Section>
            )}

            <Section kicker="EXECUTION TRACE" title="Every instruction, fully decoded">
              <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
                <MetaStrip result={result} />
                {instructions.length > 0 && <InstructionList instructions={instructions} />}
              </div>
            </Section>
          </>
        ) : null}
      </div>
    </>
  );
}
