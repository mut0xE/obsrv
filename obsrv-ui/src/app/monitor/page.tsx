'use client'

import { useState, useEffect } from 'react'
import { Eye, Code, Trash2, Plus, ExternalLink } from 'lucide-react'
import { api } from '@/lib/api'
import { Input, Button, Spinner, Panel } from '@/components/ui'
import { ErrorDisplay, EmptyState } from '@/components/error-display'

export default function MonitorPage() {
  const [tab, setTab] = useState<'wallets' | 'programs'>('wallets')
  const [monitors, setMonitors] = useState<any>({ wallets: [], programs: [] })
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<Error | null>(null)

  const fetchMonitors = async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.listMonitors()
      setMonitors(data)
    } catch (err) {
      setError(err as Error)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    fetchMonitors()
  }, [])

  return (
    <div className="animate-fadein">
      <div className="mb-10">
        <div className="font-mono text-[11px] uppercase tracking-widest mb-3" style={{ color: 'var(--text-tertiary)' }}>
          Real-time Monitoring
        </div>
        <h1 className="text-[30px] font-semibold mb-3" style={{ color: 'var(--text-primary)', letterSpacing: '-0.025em' }}>
          Monitor
        </h1>
        <p className="text-sm leading-relaxed max-w-[660px]" style={{ color: 'var(--text-secondary)' }}>
          Track Solana wallets and programs in real-time. Get instant Telegram alerts when risk exceeds your threshold.
        </p>
      </div>

      {/* Telegram CTA */}
      <div className="p-6 mb-8 border" style={{ background: 'var(--info-dim)', borderColor: 'var(--info)' }}>
        <div className="flex items-center gap-4">
          <div className="flex-1">
            <p className="text-sm font-medium mb-1" style={{ color: 'var(--text-primary)' }}>
              Set up monitoring via Telegram
            </p>
            <p className="text-xs leading-relaxed" style={{ color: 'var(--text-secondary)' }}>
              Message your bot on Telegram to configure monitoring. The bot will guide you through wallet setup and provide your chat ID automatically.
            </p>
          </div>
          <a
            href="https://t.me/YOUR_BOT_USERNAME"
            target="_blank"
            rel="noopener noreferrer"
            className="px-5 py-3 font-mono text-[13px] font-medium uppercase tracking-widest flex items-center gap-2 transition-opacity hover:opacity-80"
            style={{ background: 'var(--info)', color: 'var(--text-primary)' }}
          >
            Open Bot <ExternalLink size={12} />
          </a>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2 mb-6 border-b" style={{ borderColor: 'var(--bg-border)' }}>
        <button
          onClick={() => setTab('wallets')}
          className="px-6 py-4 font-mono text-[13px] uppercase tracking-widest relative transition-colors"
          style={{
            color: tab === 'wallets' ? 'var(--gold)' : 'var(--text-tertiary)',
            marginBottom: '-1px',
          }}
        >
          {tab === 'wallets' && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5" style={{ background: 'var(--gold)' }} />
          )}
          <Eye size={14} className="inline mr-2" />
          Wallets ({monitors.wallets?.filter((w: any) => w.active).length || 0})
        </button>
        <button
          onClick={() => setTab('programs')}
          className="px-6 py-4 font-mono text-[13px] uppercase tracking-widest relative transition-colors"
          style={{
            color: tab === 'programs' ? 'var(--gold)' : 'var(--text-tertiary)',
            marginBottom: '-1px',
          }}
        >
          {tab === 'programs' && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5" style={{ background: 'var(--gold)' }} />
          )}
          <Code size={14} className="inline mr-2" />
          Programs ({monitors.programs?.filter((p: any) => p.active).length || 0})
        </button>
      </div>

      {error && (
        <div className="mb-6">
          <ErrorDisplay error={error} onRetry={fetchMonitors} title="Failed to load monitors" />
        </div>
      )}

      {loading ? (
        <div className="flex justify-center py-20">
          <Spinner size={24} />
        </div>
      ) : tab === 'wallets' ? (
        <WalletSection wallets={monitors.wallets || []} onRefresh={fetchMonitors} />
      ) : (
        <ProgramSection programs={monitors.programs || []} onRefresh={fetchMonitors} />
      )}
    </div>
  )
}

function WalletSection({ wallets, onRefresh }: { wallets: any[]; onRefresh: () => void }) {
  const [showForm, setShowForm] = useState(false)
  const [formWallet, setFormWallet] = useState('')
  const [formChatId, setFormChatId] = useState('')
  const [formThreshold, setFormThreshold] = useState(7)
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const active = wallets.filter((w) => w.active)

  const handleAdd = async () => {
    if (!formWallet.trim() || !formChatId.trim()) return
    setSubmitting(true)
    setError(null)
    try {
      await api.addWallet(formWallet.trim(), formChatId.trim(), formThreshold)
      setFormWallet('')
      setFormChatId('')
      setShowForm(false)
      onRefresh()
    } catch (err) {
      setError(err as Error)
    } finally {
      setSubmitting(false)
    }
  }

  const handleRemove = async (wallet: string) => {
    try {
      await api.removeWallet(wallet)
      onRefresh()
    } catch (err) {
      console.error('Failed to remove wallet:', err)
    }
  }

  return (
    <div className="space-y-4">
      {active.map((w) => (
        <Panel key={w.wallet} className="px-6 py-5 flex items-center gap-4">
          <div className="flex-1 min-w-0">
            <div className="font-mono text-sm mb-2" style={{ color: 'var(--text-primary)' }}>
              {w.wallet}
            </div>
            <div className="flex items-center gap-4 text-xs" style={{ color: 'var(--text-tertiary)' }}>
              <span>Threshold: {w.alert_threshold}/10</span>
              <span>Chat: {w.telegram_chat_id}</span>
            </div>
          </div>
          <button
            onClick={() => handleRemove(w.wallet)}
            className="p-2 transition-opacity hover:opacity-70"
            style={{ color: 'var(--critical)' }}
          >
            <Trash2 size={16} />
          </button>
        </Panel>
      ))}

      {active.length === 0 && !showForm && <EmptyState message="No wallets being monitored" />}

      {error && (
        <div className="mt-4">
          <ErrorDisplay error={error} onRetry={() => setError(null)} title="Failed to add wallet" />
        </div>
      )}

      {showForm ? (
        <Panel className="px-6 py-6 space-y-4">
          <Input value={formWallet} onChange={setFormWallet} placeholder="Wallet address (base58)" />
          <Input value={formChatId} onChange={setFormChatId} placeholder="Telegram chat ID" />
          <div>
            <label className="text-xs mb-2 block" style={{ color: 'var(--text-tertiary)' }}>
              Alert threshold: {formThreshold}/10
            </label>
            <input
              type="range"
              min={1}
              max={10}
              value={formThreshold}
              onChange={(e) => setFormThreshold(Number(e.target.value))}
              className="w-full"
            />
          </div>
          <div className="flex gap-3">
            <Button onClick={handleAdd} disabled={submitting}>
              {submitting ? <Spinner size={12} /> : 'Add Wallet'}
            </Button>
            <Button variant="ghost" onClick={() => setShowForm(false)}>
              Cancel
            </Button>
          </div>
        </Panel>
      ) : (
        <Button variant="ghost" onClick={() => setShowForm(true)} className="w-full">
          <Plus size={14} className="inline mr-2" />
          Add Wallet
        </Button>
      )}
    </div>
  )
}

function ProgramSection({ programs, onRefresh }: { programs: any[]; onRefresh: () => void }) {
  const [showForm, setShowForm] = useState(false)
  const [formProgram, setFormProgram] = useState('')
  const [formName, setFormName] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  const active = programs.filter((p) => p.active)

  const handleAdd = async () => {
    if (!formProgram.trim()) return
    setSubmitting(true)
    setError(null)
    try {
      await api.addProgram(formProgram.trim(), formName.trim() || undefined)
      setFormProgram('')
      setFormName('')
      setShowForm(false)
      onRefresh()
    } catch (err) {
      setError(err as Error)
    } finally {
      setSubmitting(false)
    }
  }

  const handleRemove = async (program_id: string) => {
    try {
      await api.removeProgram(program_id)
      onRefresh()
    } catch (err) {
      console.error('Failed to remove program:', err)
    }
  }

  return (
    <div className="space-y-4">
      {active.map((p) => (
        <Panel key={p.program_id} className="px-6 py-5 flex items-center gap-4">
          <div className="flex-1 min-w-0">
            <div className="font-mono text-sm mb-1" style={{ color: 'var(--text-primary)' }}>
              {p.name && <span className="mr-3" style={{ color: 'var(--gold)' }}>{p.name}</span>}
              {p.program_id}
            </div>
            <div className="text-xs" style={{ color: 'var(--text-tertiary)' }}>
              Added {new Date(p.created_at * 1000).toLocaleDateString()}
            </div>
          </div>
          <button
            onClick={() => handleRemove(p.program_id)}
            className="p-2 transition-opacity hover:opacity-70"
            style={{ color: 'var(--critical)' }}
          >
            <Trash2 size={16} />
          </button>
        </Panel>
      ))}

      {active.length === 0 && !showForm && <EmptyState message="No programs being monitored" />}

      {error && (
        <div className="mt-4">
          <ErrorDisplay error={error} onRetry={() => setError(null)} title="Failed to add program" />
        </div>
      )}

      {showForm ? (
        <Panel className="px-6 py-6 space-y-4">
          <Input value={formProgram} onChange={setFormProgram} placeholder="Program ID (base58)" />
          <Input value={formName} onChange={setFormName} placeholder="Label (optional, e.g. 'Jupiter v6')" />
          <div className="flex gap-3">
            <Button onClick={handleAdd} disabled={submitting}>
              {submitting ? <Spinner size={12} /> : 'Add Program'}
            </Button>
            <Button variant="ghost" onClick={() => setShowForm(false)}>
              Cancel
            </Button>
          </div>
        </Panel>
      ) : (
        <Button variant="ghost" onClick={() => setShowForm(true)} className="w-full">
          <Plus size={14} className="inline mr-2" />
          Add Program
        </Button>
      )}
    </div>
  )
}
