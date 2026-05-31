"use client";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";

export class ApiError extends Error {
  constructor(
    message: string,
    public status?: number,
    public data?: any,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

let requestCount = 0;

async function request<T>(
  path: string,
  options: RequestInit = {},
  retries = 1,
): Promise<T> {
  const requestId = ++requestCount;
  const method = options.method || "GET";

  try {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), 15000);

    const res = await fetch(`${API_BASE}${path}`, {
      headers: { "Content-Type": "application/json" },
      ...options,
      signal: controller.signal,
    });

    window.clearTimeout(timeout);

    if (!res.ok) {
      const body = await res.json().catch(() => ({ error: res.statusText }));
      throw new ApiError(
        body.error || body.message || `Request failed: ${res.status}`,
        res.status,
        body,
      );
    }

    return res.json();
  } catch (error) {
    if (error instanceof ApiError) throw error;

    const message = error instanceof Error ? error.message : "Network error";
    const isAbort = error instanceof Error && error.name === "AbortError";
    const isNetwork =
      isAbort || message.includes("fetch") || message.includes("Failed");

    if (isNetwork && retries > 0) {
      await new Promise((resolve) => window.setTimeout(resolve, 600));
      return request(path, options, retries - 1);
    }

    throw new ApiError(
      isAbort
        ? "Request timeout - API server may be offline"
        : `Cannot connect to API server at ${API_BASE}`,
      0,
      { method, requestId, networkError: !isAbort, timeout: isAbort },
    );
  }
}

function query(params: Record<string, string | number | undefined>) {
  const qs = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== "") qs.set(key, String(value));
  });
  const text = qs.toString();
  return text ? `?${text}` : "";
}

function flattenTxResponse(response: any) {
  const tx = response?.tx;
  if (!tx) return response;

  const balanceChanges = tx.balances?.changes || [];
  const solBalanceChanges = balanceChanges
    .filter((change: any) => change.sol)
    .map((change: any) => ({ address: change.address, ...change.sol }));
  const tokenBalanceChanges = balanceChanges.flatMap((change: any) =>
    (change.tokens || []).map((token: any) => ({
      owner: change.address,
      ...token,
    })),
  );

  return {
    ...response,
    ...tx.analysis,
    fee_payer: tx.meta?.fee_payer,
    is_durable_nonce: tx.meta?.is_durable_nonce,
    nonce: tx.meta?.nonce,
    instructions: (tx.instructions || []).map((ix: any) => ({
      ...ix,
      program_name: ix.program,
      program_id: ix.program,
      instruction_type: ix.type,
      decoded_args: ix.accounts,
    })),
    sol_balance_changes: solBalanceChanges,
    token_balance_changes: tokenBalanceChanges,
    execution_status:
      tx.forensics?.execution_status ||
      (tx.simulation
        ? tx.simulation.success
          ? "success"
          : "failed"
        : undefined),
    failure_reason: tx.forensics?.failure_reason || tx.simulation?.error,
    cu_consumed: tx.forensics?.cu_consumed || tx.simulation?.compute?.consumed,
    fee_lamports: tx.forensics?.fee?.lamports || tx.simulation?.fee?.lamports,
    fee_sol: tx.forensics?.fee?.sol || tx.simulation?.fee?.sol,
    logs: tx.forensics?.logs || tx.simulation?.logs,
    raw: tx,
  };
}

function normalizeWalletAnalytics(data: any) {
  return {
    ...data,
    wallet_stats: data.summary,
    program_stats: data.programs || [],
    instruction_stats: data.instructions || [],
  };
}

function normalizeStats(data: any) {
  return {
    ...data,
    total_txs: data.total_transactions || 0,
    total_alerts: 0,
    avg_risk_score: 0,
    critical_count: 0,
    safe_count: data.total_transactions || 0,
    warning_count: 0,
  };
}

function normalizeMonitorList(data: any) {
  const wallets = (data?.wallets || []).map((wallet: any) => ({
    ...wallet,
    address: wallet.address || wallet.wallet,
    wallet: wallet.wallet || wallet.address,
    name: wallet.name || wallet.label,
    added_at:
      wallet.added_at ||
      (wallet.created_at ? wallet.created_at * 1000 : undefined),
  }));

  const programs = (data?.programs || []).map((program: any) => ({
    ...program,
    address: program.address || program.program_id,
    program_id: program.program_id || program.address,
    label: program.label || program.name,
    added_at:
      program.added_at ||
      (program.created_at ? program.created_at * 1000 : undefined),
  }));

  return { ...data, wallets, programs };
}

export const api = {
  health: () => request("/health"),

  analyze: async (signature: string) =>
    flattenTxResponse(
      await request("/analyze", {
        method: "POST",
        body: JSON.stringify({ signature }),
      }),
    ),

  forensics: async (signature: string) =>
    flattenTxResponse(
      await request("/forensics", {
        method: "POST",
        body: JSON.stringify({ signature }),
      }),
    ),

  simulate: async (raw_tx: string) =>
    flattenTxResponse(
      await request("/simulate", {
        method: "POST",
        body: JSON.stringify({ raw_tx }),
      }),
    ),

  nonceInspect: (nonce_account: string) =>
    request("/nonce/inspect", {
      method: "POST",
      body: JSON.stringify({ nonce_account }),
    }),

  addWallet: (wallet: string, telegram_chat_id?: string, alert_threshold = 7) =>
    request("/monitor/wallet", {
      method: "POST",
      body: JSON.stringify({
        wallet,
        telegram_chat_id: telegram_chat_id || undefined,
        alert_threshold,
      }),
    }),

  removeWallet: (wallet: string) =>
    request("/monitor/wallet/remove", {
      method: "POST",
      body: JSON.stringify({ wallet }),
    }),

  addProgram: (program_id: string, name?: string) =>
    request("/monitor/program", {
      method: "POST",
      body: JSON.stringify({ program_id, name }),
    }),

  removeProgram: (program_id: string) =>
    request("/monitor/program/remove", {
      method: "POST",
      body: JSON.stringify({ program_id }),
    }),

  listMonitors: async () =>
    normalizeMonitorList(await request("/monitor/list")),
  getMonitorList: async () =>
    normalizeMonitorList(await request("/monitor/list")),

  walletAnalytics: async (wallet: string) =>
    normalizeWalletAnalytics(
      await request(`/analytics/wallet${query({ wallet })}`),
    ),
  getWalletAnalytics: async (wallet: string) =>
    normalizeWalletAnalytics(
      await request(`/analytics/wallet${query({ wallet })}`),
    ),

  programAnalytics: (program_id: string) =>
    request(`/analytics/program${query({ program_id })}`),
  getProgramAnalytics: (program_id: string) =>
    request(`/analytics/program${query({ program_id })}`),

  topPrograms: (limit?: number) =>
    request(`/analytics/programs${query({ limit })}`),
  getTopPrograms: (limit?: number) =>
    request(`/analytics/programs${query({ limit })}`).then(
      (res: any) => res.programs || [],
    ),

  alerts: (wallet?: string, limit?: number) =>
    request(`/analytics/alerts${query({ wallet, limit })}`),
  getAlerts: (limit?: number, offset?: number) =>
    request(`/analytics/alerts${query({ limit, offset })}`).then(
      (res: any) => res.alerts || [],
    ),

  getTransactions: (
    limitOrParams?: number | Record<string, any>,
    offset?: number,
  ) => {
    const params =
      typeof limitOrParams === "number"
        ? { limit: limitOrParams, offset }
        : limitOrParams;
    return request(`/stream/transactions${query(params || {})}`);
  },

  getTransaction: (signature: string) =>
    request(`/stream/tx${query({ signature })}`),
  getStats: async () => normalizeStats(await request("/stream/stats")),
  getWalletHistory: (wallet: string, limit?: number) =>
    request(`/stream/wallet/history${query({ wallet, limit })}`),
};

export function connectWs(onMessage?: (data: any) => void): WebSocket {
  const wsUrl =
    API_BASE.replace(/^https:/, "wss:").replace(/^http:/, "ws:") + "/ws";
  const ws = new WebSocket(wsUrl);

  ws.onmessage = (e) => {
    try {
      onMessage?.(JSON.parse(e.data));
    } catch (err) {
      console.error("WS parse error:", err);
    }
  };

  ws.onerror = (err) => {
    console.error("WS error:", err);
  };

  ws.onclose = () => {
    window.setTimeout(() => connectWs(onMessage), 3000);
  };

  return ws;
}
