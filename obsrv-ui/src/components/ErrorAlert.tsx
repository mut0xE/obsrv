'use client';

import React from 'react';

interface AlertState {
  id: string;
  type: 'error' | 'warning' | 'success' | 'info';
  message: string;
  detail?: string;
  timestamp: number;
}

let alertId = 0;
const alerts: Map<string, AlertState> = new Map();
const listeners: Set<(alerts: AlertState[]) => void> = new Set();

export function useAlerts() {
  const [alertList, setAlertList] = React.useState<AlertState[]>([]);

  React.useEffect(() => {
    const handler = (list: AlertState[]) => setAlertList(list);
    listeners.add(handler);
    return () => {
      listeners.delete(handler);
    };
  }, []);

  return {
    alerts: alertList,
    showError: (message: string, detail?: string) => {
      const id = String(alertId++);
      const alert: AlertState = {
        id,
        type: 'error',
        message,
        detail,
        timestamp: Date.now(),
      };
      alerts.set(id, alert);
      notify();
      setTimeout(() => removeAlert(id), 6000);
      return id;
    },
    showSuccess: (message: string) => {
      const id = String(alertId++);
      const alert: AlertState = {
        id,
        type: 'success',
        message,
        timestamp: Date.now(),
      };
      alerts.set(id, alert);
      notify();
      setTimeout(() => removeAlert(id), 4000);
      return id;
    },
    showWarning: (message: string, detail?: string) => {
      const id = String(alertId++);
      const alert: AlertState = {
        id,
        type: 'warning',
        message,
        detail,
        timestamp: Date.now(),
      };
      alerts.set(id, alert);
      notify();
      setTimeout(() => removeAlert(id), 5000);
      return id;
    },
    removeAlert: (id: string) => removeAlert(id),
  };
}

function notify() {
  const list = Array.from(alerts.values());
  listeners.forEach((cb) => cb(list));
}

function removeAlert(id: string) {
  alerts.delete(id);
  notify();
}

export function AlertContainer() {
  const { alerts } = useAlerts();

  return (
    <div
      style={{
        position: 'fixed',
        top: '20px',
        right: '20px',
        zIndex: 9999,
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        maxWidth: '440px',
      }}
    >
      {alerts.map((alert) => (
        <Alert key={alert.id} alert={alert} />
      ))}
    </div>
  );
}

function Alert({ alert }: { alert: AlertState }) {
  const colors: Record<string, { bg: string; border: string; text: string; icon: string }> = {
    error: {
      bg: 'var(--critical-dim)',
      border: 'var(--critical)',
      text: 'var(--critical)',
      icon: '✕',
    },
    warning: {
      bg: 'var(--warning-dim)',
      border: 'var(--warning)',
      text: 'var(--warning)',
      icon: '⚠',
    },
    success: {
      bg: 'var(--safe-dim)',
      border: 'var(--safe)',
      text: 'var(--safe)',
      icon: '✓',
    },
    info: {
      bg: 'var(--info-dim)',
      border: 'var(--info)',
      text: 'var(--info)',
      icon: 'ℹ',
    },
  };

  const color = colors[alert.type];

  return (
    <div
      style={{
        background: color.bg,
        border: `1px solid ${color.border}`,
        borderRadius: '4px',
        padding: '14px 16px',
        display: 'grid',
        gridTemplateColumns: '20px 1fr',
        gap: '12px',
        alignItems: 'flex-start',
        animation: 'slidefade 220ms ease-out',
      }}
    >
      <div style={{ color: color.text, fontSize: '14px', fontWeight: '600', marginTop: '1px' }}>
        {color.icon}
      </div>
      <div style={{ minWidth: 0 }}>
        <div style={{ color: color.text, fontSize: '13px', fontFamily: 'var(--font-mono)', fontWeight: '500', marginBottom: alert.detail ? '4px' : 0 }}>
          {alert.message}
        </div>
        {alert.detail && (
          <div style={{ color: color.text, fontSize: '12px', opacity: 0.8, fontFamily: 'var(--font-mono)', lineHeight: '1.4' }}>
            {alert.detail}
          </div>
        )}
      </div>
    </div>
  );
}
