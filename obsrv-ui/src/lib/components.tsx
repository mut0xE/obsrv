"use client";

import React from "react";

// Sparkline chart
export function Sparkline({
  data,
  color = "var(--info)",
  width = 72,
  height = 22,
  fillOpacity = 0.18,
}: {
  data: number[];
  color?: string;
  width?: number;
  height?: number;
  fillOpacity?: number;
}) {
  if (!data || data.length === 0) return null;
  const max = Math.max(...data);
  const min = Math.min(...data);
  const range = max - min || 1;
  const stepX = width / (data.length - 1);
  const pts = data.map(
    (v, i) =>
      `${(i * stepX).toFixed(1)},${(height - ((v - min) / range) * (height - 4) - 2).toFixed(1)}`,
  );
  const path = "M " + pts.join(" L ");
  const fill = `M 0,${height} L ` + pts.join(" L ") + ` L ${width},${height} Z`;
  return (
    <svg
      className="sparkline"
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
    >
      <path d={fill} fill={color} opacity={fillOpacity} />
      <path
        d={path}
        stroke={color}
        strokeWidth="1.2"
        fill="none"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// Sparkbars chart
export function SparkBars({
  data,
  color = "var(--info)",
  width = 72,
  height = 22,
}: {
  data: number[];
  color?: string;
  width?: number;
  height?: number;
}) {
  const max = Math.max(...data) || 1;
  const barW = width / data.length;
  return (
    <svg
      className="sparkline"
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
    >
      {data.map((v, i) => {
        const h = Math.max(1.5, (v / max) * (height - 1));
        return (
          <rect
            key={i}
            x={i * barW + 0.5}
            y={height - h}
            width={barW - 1.5}
            height={h}
            fill={color}
            opacity="0.9"
          />
        );
      })}
    </svg>
  );
}

// Risk Badge
export function RiskBadge({
  level,
  children,
  size = "md",
}: {
  level: "critical" | "warning" | "safe" | "info" | "neutral";
  children: React.ReactNode;
  size?: "sm" | "md";
}) {
  return (
    <span className={`risk-badge ${level} ${size === "sm" ? "sm" : ""}`}>
      {children}
    </span>
  );
}

// Program Pill
export function ProgramPill({
  name,
  type = "system",
  short,
}: {
  name: string;
  type?: "system" | "token" | "compute" | "unknown" | "serum";
  short?: string;
}) {
  const map: Record<string, string> = {
    system: "S",
    token: "T",
    compute: "C",
    unknown: "?",
    serum: "X",
  };
  const letter = short || map[type] || "?";
  return (
    <span className={`program-pill ${type}`}>
      <span className="program-pill-icon">{letter}</span>
      <span>{name}</span>
    </span>
  );
}

// Address Display
export function AddressDisplay({
  address,
  label,
  showCopy = true,
  full,
}: {
  address?: string | null;
  label?: string;
  showCopy?: boolean;
  full?: boolean;
}) {
  const safeAddress = address || "—";
  const display = full
    ? safeAddress
    : safeAddress.length > 12
      ? `${safeAddress.slice(0, 4)}…${safeAddress.slice(-4)}`
      : safeAddress;
  const [copied, setCopied] = React.useState(false);

  function copy(e: React.MouseEvent) {
    e.stopPropagation();
    if (!address) return;
    navigator.clipboard?.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  }

  return (
    <span className="address" title={safeAddress}>
      <span>{display}</span>
      {label ? <span className="addr-label">{label}</span> : null}
      {showCopy && address ? (
        <span className="addr-copy" onClick={copy}>
          {copied ? "✓" : "⎘"}
        </span>
      ) : null}
    </span>
  );
}

// Sol Amount
export function SolAmount({
  value,
  signed = false,
  sym = "SOL",
}: {
  value: number;
  signed?: boolean;
  sym?: string;
}) {
  const sign = value > 0 ? "+" : value < 0 ? "−" : "";
  const abs = Math.abs(value);
  const cls = value > 0 ? "positive" : value < 0 ? "negative" : "";
  const formatted = abs.toLocaleString("en-US", {
    maximumFractionDigits: 9,
    minimumFractionDigits: abs < 1 ? Math.min(6, 9) : 4,
  });
  return (
    <span className={`sol-amount ${cls}`}>
      <span className="num">
        {signed ? sign : ""}
        {formatted}
      </span>
      <span className="sym">{sym}</span>
    </span>
  );
}

// Live Bar
export function LiveBar({
  value,
  max = 100,
  showPct = true,
}: {
  value: number;
  max?: number;
  showPct?: boolean;
}) {
  const pct = Math.min(100, (value / max) * 100);
  const [drawn, setDrawn] = React.useState(0);

  React.useEffect(() => {
    const t = setTimeout(() => setDrawn(pct), 60);
    return () => clearTimeout(t);
  }, [pct]);

  let cls = "";
  if (pct >= 80) cls = "critical";
  else if (pct >= 50) cls = "warning";

  return (
    <div className="live-bar">
      <div className="live-bar-track">
        <div
          className={`live-bar-fill ${cls}`}
          style={{ width: drawn + "%" }}
        />
      </div>
      {showPct ? <div className="live-bar-pct">{Math.round(pct)}%</div> : null}
    </div>
  );
}

// Pulse Dot
export function PulseDot({ color = "safe" }: { color?: string }) {
  return <span className={`dot ${color}`} />;
}

// Static Dot
export function StaticDot({ color = "tertiary" }: { color?: string }) {
  return <span className={`dot static ${color}`} />;
}

// Stat Card
export function StatCard({
  label,
  value,
  sub,
  accent,
  footer,
  children,
}: {
  label: string;
  value: string | React.ReactNode;
  sub?: string;
  accent?: string;
  footer?: React.ReactNode;
  children?: React.ReactNode;
}) {
  const accentColor =
    accent === "safe"
      ? "var(--safe)"
      : accent === "critical"
        ? "var(--critical)"
        : accent === "warning"
          ? "var(--warning)"
          : "var(--text-primary)";
  return (
    <div className="stat-card">
      <div className="stat-card-label">{label}</div>
      <div
        className="stat-card-value"
        style={accent ? { color: accentColor } : undefined}
      >
        {value}
      </div>
      {sub ? <div className="stat-card-sub">{sub}</div> : null}
      {footer ? <div style={{ marginTop: 10 }}>{footer}</div> : null}
      {children}
    </div>
  );
}

// Clock component
export function Clock() {
  const [t, setT] = React.useState(() => new Date());

  React.useEffect(() => {
    const id = setInterval(() => setT(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const p = (n: number) => String(n).padStart(2, "0");

  return (
    <span style={{ fontVariantNumeric: "tabular-nums" }}>
      {p(t.getUTCHours())}:{p(t.getUTCMinutes())}:{p(t.getUTCSeconds())}{" "}
      <span style={{ color: "var(--text-tertiary)" }}>UTC</span>
    </span>
  );
}
