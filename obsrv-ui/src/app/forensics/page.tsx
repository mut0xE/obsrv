'use client'

import { useState } from 'react'
import { FileText } from 'lucide-react'
import { api } from '@/lib/api'
import { Input, Button, Spinner, RiskBadge, ProgramPill, AddressDisplay, Panel, StatCard } from '@/components/ui'
import { ErrorDisplay, EmptyState } from '@/components/error-display'

export default function ForensicsPage() {
  const [signature, setSignature] = useState('')
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState<any>(null)
  const [error, setError] = useState<Error | null>(null)

  const handleFetch = async () => {
    if (!signature.trim()) return

    setLoading(true)
    setError(null)
    setResult(null)

    try {
      const data = await api.forensics(signature.trim())
      setResult(data)
    } catch (err) {
      setError(err as Error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="animate-fadein">
      <div className="mb-10">
        <div className="font-mono text-[11px] uppercase tracking-widest mb-3" style={{ color: 'var(--text-tertiary)' }}>
          Transaction Forensics
        </div>
        <h1 className="text-[30px] font-semibold mb-3" style={{ color: 'var(--text-primary)', letterSpacing: '-0.025em' }}>
          Deep Inspection
        </h1>
        <p className="text-sm leading-relaxed max-w-[660px]" style={{ color: 'var(--text-secondary)' }}>
          Fetch and inspect a transaction with full decoded instructions and detailed balance analysis.
        </p>
      </div>

      <div className="grid gap-3 mb-8" style={{ gridTemplateColumns: '1fr auto' }}>
        <Input
          value={signature}
          onChange={setSignature}
          placeholder="Transaction signature..."
          onKeyDown={(e) => e.key === 'Enter' && handleFetch()}
        />
        <Button onClick={handleFetch} disabled={loading || !signature.trim()}>
          {loading ? <><Spinner size={14} /> Fetching...</> : <><FileText size={14} className="inline mr-2" />Inspect</>}
        </Button>
      </div>

      {error && <div className="mb-8"><ErrorDisplay error={error} onRetry={handleFetch} title="Forensics Failed" /></div>}

      {result && <ForensicsResult data={result.data || result} />}

      {!result && !error && !loading && <EmptyState message="Enter a transaction signature to begin forensic inspection" />}
    </div>
  )
}

function ForensicsResult({ data }: { data: any }) {
  return (
    <div className="space-y-12 animate-slidefade">
      <div className="grid grid-cols-4 gap-4">
        <StatCard label="Risk Score" value={<RiskBadge score={data.risk_score ?? 0} size="sm" />} />
        <StatCard label="Fee" value={data.fee_lamports ? `${(data.fee_lamports / 1e9).toFixed(6)} SOL` : '—'} />
        <StatCard label="Compute Units" value={data.cu_consumed?.toLocaleString() || '—'} />
        <StatCard label="Slot" value={data.slot?.toLocaleString() || '—'} />
      </div>

      <Panel className="px-8 py-6">
        <h3 className="text-sm font-medium mb-3" style={{ color: 'var(--text-tertiary)' }}>Summary</h3>
        <p className="text-sm leading-relaxed mb-4" style={{ color: 'var(--text-primary)' }}>
          {data.summary || 'No summary available'}
        </p>
        {data.programs_called && data.programs_called.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {data.programs_called.map((p: any, i: number) => (
              <ProgramPill key={i} name={p.name} programId={p.id} />
            ))}
          </div>
        )}
      </Panel>

      {data.sol_balance_changes && data.sol_balance_changes.length > 0 && (
        <div>
          <h2 className="text-[19px] font-medium mb-4" style={{ color: 'var(--text-secondary)' }}>
            SOL Balance Changes
          </h2>
          <Panel>
            <table className="w-full">
              <thead style={{ background: 'var(--bg-surface)' }}>
                <tr>
                  <th className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>Address</th>
                  <th className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>Before</th>
                  <th className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>After</th>
                  <th className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>Change</th>
                </tr>
              </thead>
              <tbody>
                {data.sol_balance_changes.map((c: any, i: number) => (
                  <tr key={i} className="border-t group" style={{ borderColor: 'var(--bg-border)' }}>
                    <td className="px-6 py-4"><AddressDisplay address={c.address} /></td>
                    <td className="px-6 py-4 text-right font-data text-sm" style={{ color: 'var(--text-tertiary)' }}>{c.before_sol?.toFixed(4)}</td>
                    <td className="px-6 py-4 text-right font-data text-sm" style={{ color: 'var(--text-tertiary)' }}>{c.after_sol?.toFixed(4)}</td>
                    <td className="px-6 py-4 text-right font-data" style={{ color: c.change_sol < 0 ? 'var(--critical)' : 'var(--safe)' }}>
                      {c.change_sol >= 0 ? '+' : ''}{c.change_sol?.toFixed(6)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        </div>
      )}
    </div>
  )
}
