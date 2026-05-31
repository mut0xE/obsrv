'use client';

import React, { useState, useEffect } from 'react';
import { Clock, PulseDot } from '@/lib/components';

const NAV = [
  { key: 'analyze', label: 'Analyze', glyph: '◆', shortcut: '1' },
  { key: 'forensics', label: 'Forensics', glyph: '◇', shortcut: '2' },
  { key: 'monitor', label: 'Monitor', glyph: '◈', shortcut: '3' },
  { key: 'analytics', label: 'Analytics', glyph: '◉', shortcut: '4' },
];

function Sidebar({ active, onChange }: { active: string; onChange: (key: string) => void }) {
  return (
    <aside className="sidebar">
      <div className="sidebar-logo">
        <div className="sidebar-logo-mark" />
        <span className="sidebar-logo-text">obsrv</span>
      </div>

      <nav className="sidebar-nav">
        {NAV.map((item) => (
          <div
            key={item.key}
            className={`nav-item ${active === item.key ? 'active' : ''}`}
            onClick={() => onChange(item.key)}
            style={{ cursor: 'pointer' }}
          >
            <span className="nav-item-icon">{item.glyph}</span>
            <span>{item.label}</span>
            <span className="nav-item-shortcut">⌘{item.shortcut}</span>
          </div>
        ))}
      </nav>

      <div className="sidebar-spacer" />

      <div className="sidebar-foot">
        <div className="sidebar-conn">
          <span className="dot" />
          <span>RPC · helius·mb</span>
        </div>
        <div className="sidebar-conn">
          <span className="dot" />
          <span>WSS · 4 subscribed</span>
        </div>
        <div className="sidebar-user">
          <div className="user-avatar">JR</div>
          <div>
            <div className="user-name">j.reyes</div>
            <div className="user-role">analyst · pro</div>
          </div>
        </div>
      </div>
    </aside>
  );
}

function Topbar() {
  return (
    <div className="topbar">
      <div className="topbar-search">
        <span>⌕</span>
        <input placeholder="search signature, address, or program…" />
        <span className="kbd">⌘K</span>
      </div>
      <div className="topbar-right">
        <Clock />
        <span style={{ width: 1, height: 16, background: 'var(--bg-border-strong)' }} />
        <span className="pill">
          <span className="dot" />
          MAINNET-BETA
        </span>
      </div>
    </div>
  );
}

export function AppShell({ children }: { children: React.ReactNode }) {
  const [page, setPage] = useState('analyze');

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && ['1', '2', '3', '4'].includes(e.key)) {
        e.preventDefault();
        setPage(NAV[parseInt(e.key) - 1].key);
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, []);

  return (
    <div className="app">
      <Sidebar active={page} onChange={setPage} />
      <div className="main">
        <Topbar />
        <div className="page">{children}</div>
      </div>
    </div>
  );
}
