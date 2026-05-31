"use client";

import { useState, useEffect } from "react";
import { BarChart3, Activity, TrendingUp } from "lucide-react";
import { api } from "@/lib/api";
import {
  Input,
  Button,
  Spinner,
  Panel,
  StatCard,
  AddressDisplay,
  RiskBadge,
} from "@/components/ui";
import { ErrorDisplay, EmptyState } from "@/components/error-display";

export default function AnalyticsPage() {
  const [tab, setTab] = useState<"wallet" | "programs" | "activity">("wallet");

  return (
    <div className="animate-fadein">
      <div className="mb-10">
        <div
          className="font-mono text-[11px] uppercase tracking-widest mb-3"
          style={{ color: "var(--text-tertiary)" }}
        >
          Data Insights
        </div>
        <h1
          className="text-[30px] font-semibold mb-3"
          style={{ color: "var(--text-primary)", letterSpacing: "-0.025em" }}
        >
          Analytics
        </h1>
        <p
          className="text-sm leading-relaxed max-w-[660px]"
          style={{ color: "var(--text-secondary)" }}
        >
          Aggregated statistics for monitored wallets and programs with success
          rates and usage trends.
        </p>
      </div>

      {/* Tabs */}
      <div
        className="flex gap-2 mb-8 border-b pb-2"
        style={{ borderColor: "var(--bg-border)" }}
      >
        <TabBtn
          active={tab === "wallet"}
          onClick={() => setTab("wallet")}
          icon={<Activity size={14} />}
          label="Wallet"
        />
        <TabBtn
          active={tab === "programs"}
          onClick={() => setTab("programs")}
          icon={<BarChart3 size={14} />}
          label="Programs"
        />
        <TabBtn
          active={tab === "activity"}
          onClick={() => setTab("activity")}
          icon={<TrendingUp size={14} />}
          label="Activity"
        />
      </div>

      {tab === "wallet" && <WalletTab />}
      {tab === "programs" && <ProgramsTab />}
      {tab === "activity" && <ActivityTab />}
    </div>
  );
}

function TabBtn({ active, onClick, icon, label }: any) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-2 px-6 py-3 font-mono text-[13px] uppercase tracking-widest transition-colors"
      style={{ color: active ? "var(--gold)" : "var(--text-tertiary)" }}
    >
      {icon} {label}
    </button>
  );
}

function WalletTab() {
  const [wallet, setWallet] = useState("");
  const [data, setData] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const handleSearch = async () => {
    if (!wallet.trim()) return;
    setLoading(true);
    setError(null);
    try {
      const res = await api.walletAnalytics(wallet.trim());
      setData(res);
    } catch (err) {
      setError(err as Error);
      setData(null);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="grid gap-3" style={{ gridTemplateColumns: "1fr auto" }}>
        <Input
          value={wallet}
          onChange={setWallet}
          placeholder="Enter wallet address..."
          onKeyDown={(e) => e.key === "Enter" && handleSearch()}
        />
        <Button onClick={handleSearch} disabled={loading}>
          {loading ? <Spinner size={14} /> : "Lookup"}
        </Button>
      </div>

      {error && <ErrorDisplay error={error} onRetry={handleSearch} />}

      {data && <WalletAnalyticsView data={data} />}

      {!data && !error && !loading && (
        <EmptyState message="Enter a wallet address to view analytics" />
      )}
    </div>
  );
}

function WalletAnalyticsView({ data }: { data: any }) {
  const stats = data.data?.wallet_stats || data.wallet_stats || {};
  const programs = data.data?.program_stats || data.program_stats || [];

  return (
    <div className="space-y-8 animate-slidefade">
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatCard
          label="Total Txs"
          value={stats.total_txs?.toLocaleString() || "0"}
        />
        <StatCard
          label="Failed"
          value={stats.failed_txs?.toLocaleString() || "0"}
          sub={
            stats.total_txs
              ? `${((stats.failed_txs / stats.total_txs) * 100).toFixed(1)}%`
              : "0%"
          }
        />
        <StatCard
          label="Avg CU"
          value={stats.avg_cu ? Math.round(stats.avg_cu).toLocaleString() : "0"}
        />
        <StatCard
          label="High Risk"
          value={stats.high_risk_txs?.toLocaleString() || "0"}
        />
      </div>

      {programs.length > 0 && (
        <div>
          <h2
            className="text-[19px] font-medium mb-4"
            style={{ color: "var(--text-secondary)" }}
          >
            Programs Interacted
          </h2>
          <Panel>
            <table className="w-full">
              <thead style={{ background: "var(--bg-surface)" }}>
                <tr>
                  <th
                    className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Program
                  </th>
                  <th
                    className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Calls
                  </th>
                  <th
                    className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Failed
                  </th>
                  <th
                    className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Avg CU
                  </th>
                </tr>
              </thead>
              <tbody>
                {programs.map((p: any, i: number) => (
                  <tr
                    key={i}
                    className="border-t group"
                    style={{ borderColor: "var(--bg-border)" }}
                  >
                    <td className="px-6 py-4">
                      <AddressDisplay address={p.program_id} />
                    </td>
                    <td
                      className="px-6 py-4 text-right font-data"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      {p.call_count}
                    </td>
                    <td
                      className="px-6 py-4 text-right font-data"
                      style={{
                        color:
                          p.failed_count > 0
                            ? "var(--critical)"
                            : "var(--text-tertiary)",
                      }}
                    >
                      {p.failed_count}
                    </td>
                    <td
                      className="px-6 py-4 text-right font-data"
                      style={{ color: "var(--text-tertiary)" }}
                    >
                      {Math.round(p.avg_cu || 0).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        </div>
      )}
    </div>
  );
}

function ProgramsTab() {
  const [programs, setPrograms] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    const fetchPrograms = async () => {
      setLoading(true);
      setError(null);
      try {
        const res: any = await api.topPrograms();
        setPrograms(res.data?.programs || res.programs || []);
      } catch (err) {
        setError(err as Error);
      } finally {
        setLoading(false);
      }
    };
    fetchPrograms();
  }, []);

  if (loading)
    return (
      <div className="flex justify-center py-20">
        <Spinner size={24} />
      </div>
    );
  if (error) return <ErrorDisplay error={error} />;
  if (programs.length === 0)
    return <EmptyState message="No program analytics yet" />;

  return (
    <Panel>
      <table className="w-full">
        <thead style={{ background: "var(--bg-surface)" }}>
          <tr>
            <th
              className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest"
              style={{ color: "var(--text-tertiary)" }}
            >
              #
            </th>
            <th
              className="px-6 py-3.5 text-left font-mono text-[11px] uppercase tracking-widest"
              style={{ color: "var(--text-tertiary)" }}
            >
              Program
            </th>
            <th
              className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest"
              style={{ color: "var(--text-tertiary)" }}
            >
              Calls
            </th>
            <th
              className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest"
              style={{ color: "var(--text-tertiary)" }}
            >
              Failed
            </th>
            <th
              className="px-6 py-3.5 text-right font-mono text-[11px] uppercase tracking-widest"
              style={{ color: "var(--text-tertiary)" }}
            >
              Avg CU
            </th>
          </tr>
        </thead>
        <tbody>
          {programs.map((p: any, i: number) => (
            <tr
              key={i}
              className="border-t group"
              style={{ borderColor: "var(--bg-border)" }}
            >
              <td
                className="px-6 py-4 font-mono text-sm"
                style={{ color: "var(--text-quat)" }}
              >
                {i + 1}
              </td>
              <td className="px-6 py-4">
                <div className="flex items-center gap-3">
                  {p.program_name && (
                    <span style={{ color: "var(--gold)" }}>
                      {p.program_name}
                    </span>
                  )}
                  <AddressDisplay address={p.program_id} />
                </div>
              </td>
              <td
                className="px-6 py-4 text-right font-data"
                style={{ color: "var(--text-secondary)" }}
              >
                {p.total_calls?.toLocaleString()}
              </td>
              <td
                className="px-6 py-4 text-right font-data"
                style={{
                  color:
                    p.failed_calls > 0
                      ? "var(--critical)"
                      : "var(--text-tertiary)",
                }}
              >
                {p.failed_calls?.toLocaleString()}
              </td>
              <td
                className="px-6 py-4 text-right font-data"
                style={{ color: "var(--text-tertiary)" }}
              >
                {Math.round(p.avg_cu || 0).toLocaleString()}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Panel>
  );
}

function ActivityTab() {
  const [stats, setStats] = useState<any>(null);
  const [txs, setTxs] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      setLoading(true);
      setError(null);
      try {
        const [statsRes, txsRes]: any[] = await Promise.all([
          api.getStats().catch(() => null),
          api
            .getTransactions({ limit: 20 })
            .catch(() => ({ transactions: [] })),
        ]);
        setStats(statsRes);
        setTxs(txsRes?.transactions || txsRes?.data || []);
      } catch (err) {
        setError(err as Error);
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, []);

  if (loading)
    return (
      <div className="flex justify-center py-20">
        <Spinner size={24} />
      </div>
    );
  if (error) return <ErrorDisplay error={error} />;

  const s = stats?.data || stats || {};

  return (
    <div className="space-y-8">
      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <StatCard
            label="Txs Processed"
            value={s.total_processed?.toLocaleString() || "0"}
          />
          <StatCard
            label="Last Slot"
            value={s.last_slot?.toLocaleString() || "—"}
          />
          <StatCard label="Watched Wallets" value={s.watched_wallets || "0"} />
          <StatCard
            label="Watched Programs"
            value={s.watched_programs || "0"}
          />
        </div>
      )}

      <div>
        <h2
          className="text-[19px] font-medium mb-4"
          style={{ color: "var(--text-secondary)" }}
        >
          Recent Transactions
        </h2>
        {txs.length === 0 ? (
          <EmptyState message="No transactions processed yet" />
        ) : (
          <Panel>
            <table className="w-full text-xs">
              <thead style={{ background: "var(--bg-surface)" }}>
                <tr>
                  <th
                    className="px-5 py-3 text-left font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Signature
                  </th>
                  <th
                    className="px-5 py-3 text-left font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Wallet
                  </th>
                  <th
                    className="px-5 py-3 text-right font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Risk
                  </th>
                  <th
                    className="px-5 py-3 text-right font-mono text-[11px] uppercase tracking-widest"
                    style={{ color: "var(--text-tertiary)" }}
                  >
                    Slot
                  </th>
                </tr>
              </thead>
              <tbody>
                {txs.map((tx: any, i: number) => (
                  <tr
                    key={i}
                    className="border-t group"
                    style={{ borderColor: "var(--bg-border)" }}
                  >
                    <td className="px-5 py-3 font-mono">
                      <a
                        href={`https://solscan.io/tx/${tx.signature}`}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="hover:underline"
                        style={{ color: "var(--info)" }}
                      >
                        {tx.signature?.slice(0, 16)}...
                      </a>
                    </td>
                    <td className="px-5 py-3">
                      <AddressDisplay address={tx.wallet} />
                    </td>
                    <td className="px-5 py-3 text-right">
                      <RiskBadge score={tx.risk_score || 0} size="sm" />
                    </td>
                    <td
                      className="px-5 py-3 text-right font-data"
                      style={{ color: "var(--text-tertiary)" }}
                    >
                      {tx.slot?.toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Panel>
        )}
      </div>
    </div>
  );
}
