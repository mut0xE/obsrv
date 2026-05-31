"use client";

import React from "react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { WalletConnect } from "./WalletConnect";

const NAV = [
  { key: "analyze", label: "Analyze", glyph: "◆", shortcut: "1", path: "/analyze" },
  { key: "forensics", label: "Forensics", glyph: "◇", shortcut: "2", path: "/forensics" },
  { key: "monitor", label: "Monitor", glyph: "◈", shortcut: "3", path: "/monitor" },
  { key: "analytics", label: "Analytics", glyph: "◉", shortcut: "4", path: "/analytics" },
];

function Sidebar() {
  const pathname = usePathname();
  const router = useRouter();

  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && ["1", "2", "3", "4"].includes(e.key)) {
        e.preventDefault();
        router.push(NAV[parseInt(e.key, 10) - 1].path);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [router]);

  return (
    <aside className="sidebar">
      <div className="sidebar-logo">
        <div className="sidebar-logo-mark" />
        <span className="sidebar-logo-text">obsrv</span>
      </div>

      <nav className="sidebar-nav">
        {NAV.map((item) => {
          const active = pathname?.startsWith(item.path) ?? false;
          return (
            <Link
              key={item.key}
              href={item.path}
              className={`nav-item ${active ? "active" : ""}`}
              style={{ cursor: "pointer", textDecoration: "none" }}
            >
              <span className="nav-item-icon">{item.glyph}</span>
              <span>{item.label}</span>
              <span className="nav-item-shortcut">⌘{item.shortcut}</span>
            </Link>
          );
        })}
      </nav>

      <div className="sidebar-spacer" />

      <div style={{ padding: "0 0 16px" }}>
        <WalletConnect />
      </div>
    </aside>
  );
}

function Topbar() {
  const [time, setTime] = React.useState<Date | null>(null);

  React.useEffect(() => {
    setTime(new Date());
    const id = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const p = (n: number) => String(n).padStart(2, "0");

  return (
    <div className="topbar">
      <div />
      <div className="topbar-right">
        <span style={{ fontVariantNumeric: "tabular-nums" }}>
          {time
            ? `${p(time.getUTCHours())}:${p(time.getUTCMinutes())}:${p(time.getUTCSeconds())}`
            : "00:00:00"}{" "}
          <span style={{ color: "var(--text-tertiary)" }}>UTC</span>
        </span>
        <span style={{ width: 1, height: 16, background: "var(--bg-border-strong)" }} />
        <span className="pill">
          <span className="dot" />
          MAINNET-BETA
        </span>
      </div>
    </div>
  );
}

export function AppChrome({ children }: { children: React.ReactNode }) {
  return (
    <div className="app">
      <Sidebar />
      <div className="main">
        <Topbar />
        <div className="page">{children}</div>
      </div>
    </div>
  );
}
