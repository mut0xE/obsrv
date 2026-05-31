# obsrv

> Real-time Solana transaction monitoring with risk analysis and Telegram alerts

obsrv streams live transactions from the Solana network, decodes every instruction, scores risk, and fires Telegram alerts when monitored wallets or programs exceed your threshold.

## What it does

| Feature | Description |
|---|---|
| **Analyze** | Paste a tx signature → get decoded instructions, risk score, balance changes |
| **Forensics** | Deep-dive: execution status, CU consumed, failure reason, full logs |
| **Monitor** | Watch wallets & programs → Telegram alert when risk ≥ threshold |
| **Analytics** | Aggregate stats per wallet, program, and instruction type |
| **Live feed** | WebSocket stream of transactions as they land |

## Stack

- **API** — Rust + Axum, PostgreSQL (sqlx), Yellowstone gRPC, Teloxide (Telegram)
- **UI** — Next.js 16, TypeScript, Tailwind CSS 4, Zustand
- **Infra** — Docker Compose (postgres + api + ui)

## Quick start (Docker)

```bash
cp .env.example .env
# Fill in RPC_URL and optionally TELEGRAM_BOT_TOKEN
docker compose up --build
```

- UI: http://localhost:3000
- API: http://localhost:3001
- Health: http://localhost:3001/health

## Manual setup

### Prerequisites
- Rust 1.83+
- Node.js 20+ / Bun
- PostgreSQL 16
- (Optional) Yellowstone gRPC endpoint for live streaming

### API

```bash
cd obsrv-api
cp ../.env.example .env   # edit DATABASE_URL, RPC_URL
cargo run --release
```

The server auto-runs migrations on startup.

### UI

```bash
cd obsrv-ui
cp .env.local.example .env.local   # or set NEXT_PUBLIC_API_URL=http://localhost:3001
npm install
npm run dev   # dev mode
# or: npm run build && npm start   # production
```

## Environment variables

| Variable | Required | Description |
|---|---|---|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `RPC_URL` | Yes | Solana JSON-RPC endpoint |
| `PORT` | No (3001) | API listen port |
| `GRPC_ENDPOINT` | No | Yellowstone gRPC for live streaming |
| `GRPC_X_TOKEN` | No | Yellowstone auth token |
| `TELEGRAM_BOT_TOKEN` | No | Bot token from @BotFather |
| `NEXT_PUBLIC_API_URL` | Yes (UI) | API base URL for the frontend |

## API endpoints

```
GET  /health                  — Server status
POST /analyze                 — Analyze transaction by signature
POST /forensics               — Deep forensic inspection
POST /simulate                — Simulate raw transaction
POST /nonce/inspect           — Inspect nonce account
POST /monitor/wallet          — Add wallet to watchlist
POST /monitor/wallet/remove   — Remove wallet from watchlist
POST /monitor/program         — Add program to watchlist
POST /monitor/program/remove  — Remove program from watchlist
GET  /monitor/list            — List active monitors
GET  /analytics/wallet        — Wallet statistics
GET  /analytics/programs      — Top programs by usage
GET  /analytics/alerts        — Alert history
GET  /stream/transactions     — Recent transactions
GET  /stream/stats            — Overall statistics
GET  /ws                      — WebSocket live feed
```

## Telegram alerts

1. Create a bot via [@BotFather](https://t.me/BotFather) → copy the token
2. Set `TELEGRAM_BOT_TOKEN` in your `.env`
3. In the Monitor tab, enter your Telegram chat ID when adding a wallet
4. Alerts fire when risk score ≥ your chosen threshold (1–10)

## License

MIT
