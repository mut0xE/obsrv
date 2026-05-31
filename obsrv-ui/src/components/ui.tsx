"use client";

import { Copy, Check, Loader2 } from "lucide-react";
import { useState, ReactNode } from "react";

// Risk Badge
export function RiskBadge({
  score,
  size = "md",
}: {
  score: number;
  size?: "sm" | "md";
}) {
  const level =
    score >= 8
      ? "critical"
      : score >= 5
        ? "warning"
        : score >= 3
          ? "info"
          : "safe";
  const label =
    score >= 8
      ? "Critical"
      : score >= 5
        ? "Warning"
        : score >= 3
          ? "Info"
          : "Safe";

  const padding = size === "sm" ? "px-2.5 py-1" : "px-3 py-1.5";
  const fontSize = size === "sm" ? "text-[11px]" : "text-xs";

  return (
    <span
      className={`inline-flex items-center gap-2 font-mono ${fontSize} font-medium uppercase tracking-wider border ${padding}`}
      style={{
        color: `var(--${level})`,
        background: `var(--${level}-dim)`,
        borderColor: `var(--${level})`,
      }}
    >
      <span
        className="inline-block w-1.5 h-1.5 rounded-full"
        style={{ background: `var(--${level})` }}
      />
      {score}/10 {label}
    </span>
  );
}

// Program Pill
export function ProgramPill({
  name,
  programId,
}: {
  name?: string;
  programId?: string;
}) {
  const display =
    name || (programId ? programId.slice(0, 8) + "..." : "Unknown");

  return (
    <span
      className="inline-flex items-center gap-1.5 px-3 py-1 text-xs font-mono border"
      style={{
        background: "var(--bg-surface)",
        borderColor: "var(--bg-border)",
        color: "var(--text-secondary)",
      }}
    >
      <span style={{ color: "var(--gold)" }}>◆</span>
      {display}
    </span>
  );
}

// Address Display with Copy
export function AddressDisplay({
  address,
  truncate = true,
}: {
  address: string;
  truncate?: boolean;
}) {
  const [copied, setCopied] = useState(false);

  const display = truncate
    ? `${address.slice(0, 4)}...${address.slice(-4)}`
    : address;

  const handleCopy = () => {
    navigator.clipboard.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <span
      className="inline-flex items-center gap-2 font-mono text-[13px]"
      style={{ color: "var(--text-secondary)" }}
    >
      <span>{display}</span>
      <button
        onClick={handleCopy}
        className="opacity-0 hover:opacity-100 group-hover:opacity-100 transition-opacity"
        style={{ color: "var(--text-tertiary)" }}
      >
        {copied ? <Check size={11} /> : <Copy size={11} />}
      </button>
    </span>
  );
}

// SOL Amount
export function SolAmount({ lamports }: { lamports: number }) {
  const sol = lamports / 1e9;
  const isNeg = sol < 0;

  return (
    <span
      className="font-data"
      style={{
        color: isNeg ? "var(--critical)" : "var(--safe)",
        fontVariantNumeric: "tabular-nums",
      }}
    >
      {isNeg ? "" : "+"}
      {sol.toFixed(6)} <span className="text-[10px]">SOL</span>
    </span>
  );
}

// Button
export function Button({
  children,
  onClick,
  variant = "primary",
  disabled = false,
  className = "",
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "info" | "safe" | "critical" | "ghost";
  disabled?: boolean;
  className?: string;
}) {
  const variants = {
    primary: {
      background: "var(--text-primary)",
      color: "var(--text-inverse)",
      border: "var(--text-primary)",
    },
    info: {
      background: "transparent",
      color: "var(--info)",
      border: "var(--info)",
    },
    safe: {
      background: "transparent",
      color: "var(--safe)",
      border: "var(--safe)",
    },
    critical: {
      background: "transparent",
      color: "var(--critical)",
      border: "var(--critical)",
    },
    ghost: {
      background: "transparent",
      color: "var(--text-secondary)",
      border: "transparent",
    },
  };

  const style = variants[variant];

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`px-5 py-3 font-mono text-[13px] font-medium uppercase tracking-widest border transition-all disabled:opacity-40 disabled:cursor-not-allowed hover:enabled:opacity-80 ${className}`}
      style={style}
    >
      {children}
    </button>
  );
}

// Input
export function Input({
  value,
  onChange,
  placeholder,
  className = "",
  onKeyDown,
  multiline = false,
  rows = 1,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  onKeyDown?: (e: React.KeyboardEvent) => void;
  multiline?: boolean;
  rows?: number;
}) {
  const baseStyle = {
    background: "transparent",
    borderColor: "var(--bg-border-strong)",
    color: "var(--text-primary)",
  };

  const baseClass = `w-full px-4 py-3.5 font-mono text-[13px] border focus:outline-none placeholder:text-[var(--text-quat)] ${className}`;

  if (multiline) {
    return (
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        rows={rows}
        className={baseClass}
        style={baseStyle}
        onKeyDown={onKeyDown}
      />
    );
  }

  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className={baseClass}
      style={baseStyle}
      onKeyDown={onKeyDown}
    />
  );
}

// Panel
export function Panel({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`border ${className}`}
      style={{
        background: "var(--bg-deep)",
        borderColor: "var(--bg-border)",
      }}
    >
      {children}
    </div>
  );
}

// Spinner
export function Spinner({ size = 16 }: { size?: number }) {
  return (
    <Loader2
      size={size}
      className="animate-spin"
      style={{ color: "var(--info)" }}
    />
  );
}

// Pulse Dot
export function PulseDot({ color = "var(--safe)" }: { color?: string }) {
  return (
    <span className="relative inline-flex h-2 w-2">
      <span
        className="absolute inline-flex h-full w-full rounded-full opacity-75"
        style={{
          background: color,
          animation: "pulse-dot 2.4s ease-in-out infinite",
        }}
      />
      <span
        className="relative inline-flex rounded-full h-2 w-2"
        style={{ background: color }}
      />
    </span>
  );
}

// Stat Card
export function StatCard({
  label,
  value,
  sub,
}: {
  label: string;
  value: ReactNode;
  sub?: string;
}) {
  return (
    <Panel className="px-7 py-6">
      <div
        className="font-mono text-[11px] uppercase tracking-widest mb-2"
        style={{ color: "var(--text-tertiary)" }}
      >
        {label}
      </div>
      <div
        className="text-[26px] font-data font-medium"
        style={{
          color: "var(--text-primary)",
          fontVariantNumeric: "tabular-nums",
        }}
      >
        {value}
      </div>
      {sub && (
        <div className="text-xs mt-1" style={{ color: "var(--text-tertiary)" }}>
          {sub}
        </div>
      )}
    </Panel>
  );
}
