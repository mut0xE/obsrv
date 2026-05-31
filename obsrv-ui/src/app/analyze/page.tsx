'use client'

import { useState } from 'react'
import { Search, ChevronDown, ChevronRight } from 'lucide-react'
import { api } from '@/lib/api'
import { Input, Button, Spinner, RiskBadge, ProgramPill, AddressDisplay, SolAmount, Panel } from '@/components/ui'
import { ErrorDisplay, EmptyState } from '@/components/error-display'

export default function AnalyzePage() {
  const [signature, setSignature] = useState('')
  const [loading, setLoading] = useState(false)
  const [result, setResult] = useState<any>(null)
  const [error, setError] = useState<Error | null>(null)

  const handleAnalyze = async () => {
    if (!signature.trim()) return

    setLoading(true)
    setError(null)
    setResult(null)

    try {
      const data = await api.analyze(signature.trim())
      setResult(data)
    } catch (err) {
      setError(err as Error)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="animate-fadein">
      {/* Page Head */}
      <div className="mb-10">
        <div
          className="font-mono text-[11px] uppercase tracking-widest mb-3"
          style={{ color: 'var(--text-tertiary)' }}
        >
          Transaction Analysis
        </div>
        <h1
          className="text-[30px] font-semibold mb-3"
          style={{ color: 'var(--text-primary)', letterSpacing: '-0.025em' }}
        >
          Analyze Transaction
        </h1>
        <p
          className="text-sm leading-relaxed max-w-[660px]"
          style={{ color: 'var(--text-secondary)' }}
        >
          Paste a Solana transaction signature to get a comprehensive risk analysis with decoded instructions and balance changes.
        </p>
      </div>

      {/* Input */}
      <div className="grid gap-3 mb-8" style={{ gridTemplateColumns: '1fr auto' }}>
        <Input
          value={signature}
          onChange={setSignature}
          placeholder="Enter transaction signature..."
          onKeyDown={(e) => e.key === 'Enter' && handleAnalyze()}
        />
        <Button onClick={handleAnalyze} disabled={loading || !signature.trim()}>
          {loading ? (
            <>
              <Spinner size={14} /> Scanning...
            </>
          ) : (
            <>
              <Search size={14} className="inline mr-2" />
              Scan
            </>
          )}
        </Button>
      </div>

      {/* Error */}
      {error && (
        <div className="mb-8">
          <ErrorDisplay error={error} onRetry={handleAnalyze} title="Analysis Failed" />
        </div>
      )}

      {/* Result */}
      {result && <AnalyzeResult data={result.data || result} />}

      {!result && !error && !loading && (
        <EmptyState message="Enter a transaction signature to begin analysis" />
      )}
    </div>
  )
}

function AnalyzeResult({ data }: { data: any }) {
  const score = data.risk_score ?? 0

  return (
    <div className="space-y-12 animate-slidefade">
      {/* Verdict Hero */}
      <Panel className="px-8 py-7">
        <div className="mb-6">
          <RiskBadge score={score} size="md" />
          <p className="text-sm mt-3" style={{ color: 'var(--text-secondary)' }}>
            {data.summary || 'No summary available'}
          </p>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-4 gap-4">
          <StatItem label="Fee" value={data.fee_lamports ? `${(data.fee_lamports / 1e9).toFixed(6)} SOL` : '—'} />
          <StatItem label="CU Used" value={data.cu_consumed?.toLocaleString() || '—'} />
          <StatItem label="Programs" value={data.programs_called?.length || 0} />
          <StatItem label="Status" value={data.execution_status || '—'} />
        </div>
      </Panel>

      {/* Flags */}
      {data.flags && data.flags.length > 0 && (
        <Section title="Flags">
          <div className="space-y-2">
            {data.flags.map((flag: string, i: number) => (
              <div
                key={i}
                className="px-4 py-3 border flex items-center gap-3 text-sm"
                style={{
                  background: 'var(--bg-surface)',
                  borderColor: 'var(--bg-border)',
                  color: 'var(--text-secondary)',
                }}
              >
                <span style={{ color: 'var(--warning)' }}>▲</span>
                {flag}
              </div>
            ))}
          </div>
        </Section>
      )}

      {/* Instructions */}
      {data.instructions && data.instructions.length > 0 && (
        <Section title="Instructions">
          <div className="space-y-1">
            {data.instructions.map((ix: any, i: number) => (
              <InstructionRow key={i} ix={ix} index={i} />
            ))}
          </div>
        </Section>
      )}

      {/* SOL Changes */}
      {data.sol_balance_changes && data.sol_balance_changes.length > 0 && (
        <Section title="SOL Balance Changes">
          <Panel>
            <table className="w-full">
              <thead style={{ background: 'var(--bg-surface)' }}>
                <tr>
                  <th className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>
                    Address
                  </th>
                  <th className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>
                    Change
                  </th>
                </tr>
              </thead>
              <tbody>
                {data.sol_balance_changes.map((change: any, i: number) => (
                  <tr key={i} className="border-t group" style={{ borderColor: 'var(--bg-border)' }}>
                    <td className="px-6 py-4">
                      <AddressDisplay address={change.address} />
                    </td>
                    <td className="px-6 py-4 text-right">
                      <SolAmount lamports={change.change_sol * 1e9} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        </Section>
      )}

      {/* Token Changes */}
      {data.token_balance_changes && data.token_balance_changes.length > 0 && (
        <Section title="Token Balance Changes">
          <Panel>
            <table className="w-full">
              <thead style={{ background: 'var(--bg-surface)' }}>
                <tr>
                  <th className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>
                    Owner
                  </th>
                  <th className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>
                    Mint
                  </th>
                  <th className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest" style={{ color: 'var(--text-tertiary)' }}>
                    Change
                  </th>
                </tr>
              </thead>
              <tbody>
                {data.token_balance_changes.map((change: any, i: number) => (
                  <tr key={i} className="border-t group" style={{ borderColor: 'var(--bg-border)' }}>
                    <td className="px-6 py-4"><AddressDisplay address={change.owner} /></td>
                    <td className="px-6 py-4"><AddressDisplay address={change.mint} /></td>
                    <td className="px-6 py-4 text-right font-mono text-sm" style={{ color: 'var(--text-secondary)' }}>
                      {change.change_ui}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        </Section>
      )}
    </div>
  )
}

function InstructionRow({ ix, index }: { ix: any; index: number }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <div className="border" style={{ borderColor: 'var(--bg-border)' }}>
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-[var(--bg-hover)] transition-colors"
      >
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <span className="font-mono text-[10px] w-8" style={{ color: 'var(--text-quat)' }}>
          #{index}
        </span>
        <ProgramPill name={ix.program_name} programId={ix.program_id} />
        <span className="text-sm ml-2" style={{ color: 'var(--text-secondary)' }}>
          {ix.instruction_type || 'unknown'}
        </span>
        {ix.severity && ix.severity !== 'info' && (
          <span
            className="ml-auto text-xs uppercase tracking-wide"
            style={{ color: `var(--${ix.severity === 'critical' ? 'critical' : 'warning'})` }}
          >
            {ix.severity}
          </span>
        )}
      </button>

      {expanded && (
        <div className="border-t px-6 py-4 space-y-3 text-xs animate-slidefade" style={{ borderColor: 'var(--bg-border)', background: 'var(--bg-void)' }}>
          {ix.program_id && (
            <div className="flex gap-3">
              <span className="font-mono w-24" style={{ color: 'var(--text-tertiary)' }}>Program</span>
              <AddressDisplay address={ix.program_id} truncate={false} />
            </div>
          )}

          {ix.decoded_args && Object.keys(ix.decoded_args).length > 0 && (
            <div>
              <span className="font-mono block mb-2" style={{ color: 'var(--text-tertiary)' }}>Decoded Args:</span>
              <pre className="p-3 font-mono text-[11px] overflow-x-auto" style={{ background: 'var(--bg-elevated)', color: 'var(--text-secondary)' }}>
                {JSON.stringify(ix.decoded_args, null, 2)}
              </pre>
            </div>
          )}

          {ix.accounts && ix.accounts.length > 0 && (
            <div>
              <span className="font-mono block mb-2" style={{ color: 'var(--text-tertiary)' }}>
                Accounts ({ix.accounts.length}):
              </span>
              <div className="space-y-1">
                {ix.accounts.map((acc: any, j: number) => (
                  <div key={j} className="flex items-center gap-2 pl-4">
                    <span className="font-mono text-[10px] w-6" style={{ color: 'var(--text-quat)' }}>{j}</span>
                    <AddressDisplay address={acc.pubkey || acc} />
                    {acc.is_signer && (
                      <span className="text-[9px] px-1.5 py-0.5 uppercase tracking-wide" style={{ background: 'var(--info-dim)', color: 'var(--info)' }}>
                        Signer
                      </span>
                    )}
                    {acc.is_writable && (
                      <span className="text-[9px] px-1.5 py-0.5 uppercase tracking-wide" style={{ background: 'var(--warning-dim)', color: 'var(--warning)' }}>
                        Write
                      </span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section>
      <h2
        className="text-[19px] font-medium mb-4"
        style={{ color: 'var(--text-secondary)', letterSpacing: '-0.01em' }}
      >
        {title}
      </h2>
      {children}
    </section>
  )
}

function StatItem({ label, value }: { label: string; value: string | number }) {
  return (
    <div>
      <div className="font-mono text-[11px] uppercase tracking-widest mb-1" style={{ color: 'var(--text-tertiary)' }}>
        {label}
      </div>
      <div className="font-data text-base" style={{ color: 'var(--text-primary)' }}>
        {value}
      </div>
    </div>
  )
}
