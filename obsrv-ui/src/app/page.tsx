'use client';

import React, { useState, useEffect } from 'react';
import { PageAnalyze } from '@/components/PageAnalyze';
import { PageForensics } from '@/components/PageForensics';
import { PageMonitor } from '@/components/PageMonitor';
import { PageAnalytics } from '@/components/PageAnalytics';

export default function Home() {
  const [page, setPage] = useState('analyze');

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && ['1', '2', '3', '4'].includes(e.key)) {
        e.preventDefault();
        const pages = ['analyze', 'forensics', 'monitor', 'analytics'];
        setPage(pages[parseInt(e.key) - 1]);
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
        <div className="page">
          {page === 'analyze' && <PageAnalyze />}
          {page === 'forensics' && <PageForensics />}
          {page === 'monitor' && <PageMonitor />}
          {page === 'analytics' && <PageAnalytics />}
        </div>
      </div>
    </div>
  );
}

function Sidebar({ active, onChange }: { active: string; onChange: (key: string) => void }) {
  const NAV = [
    { key: 'analyze', label: 'Analyze', glyph: '◆', shortcut: '1' },
    { key: 'forensics', label: 'Forensics', glyph: '◇', shortcut: '2' },
    { key: 'monitor', label: 'Monitor', glyph: '◈', shortcut: '3' },
    { key: 'analytics', label: 'Analytics', glyph: '◉', shortcut: '4' },
  ];

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
  const [time, setTime] = React.useState<Date | null>(null);

  React.useEffect(() => {
    setTime(new Date());
    const id = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(id);
  }, []);

  const p = (n: number) => String(n).padStart(2, '0');

  return (
    <div className="topbar">
      <div className="topbar-search">
        <span>⌕</span>
        <input placeholder="search signature, address, or program…" />
        <span className="kbd">⌘K</span>
      </div>
      <div className="topbar-right">
        <span style={{ fontVariantNumeric: 'tabular-nums' }}>
          {time ? `${p(time.getUTCHours())}:${p(time.getUTCMinutes())}:${p(time.getUTCSeconds())}` : '00:00:00'}{' '}
          <span style={{ color: 'var(--text-tertiary)' }}>UTC</span>
        </span>
        <span style={{ width: 1, height: 16, background: 'var(--bg-border-strong)' }} />
        <span className="pill">
          <span className="dot" />
          MAINNET-BETA
        </span>
      </div>
    </div>
  );
}
