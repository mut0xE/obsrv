# obsrv

Real-time Solana transaction monitoring and risk analysis platform. Analyze, monitor, and stream on-chain activity with instant alerts.

## Features

- **Transaction Analysis** - Decode and risk-score transactions before signing (1-10 scale)
- **Forensic Inspection** - Deep inspection with execution logs, balance changes, compute usage
- **Wallet/Program Monitoring** - Per-user watchlists with real-time Yellowstone gRPC streaming
- **Telegram Alerts** - Instant notifications for high-risk transactions
- **Analytics Dashboard** - Aggregate stats across wallets, programs, and instruction types
- **WebSocket Streaming** - Live transaction feed to connected clients
- **Wallet Authentication** - Solana wallet-based user scoping

## Architecture

```
obsrv-core/     Rust library - transaction decoding, risk scoring, instruction parsing
obsrv-api/      Rust backend - Axum HTTP/WS server, Yellowstone gRPC streaming, Telegram bot
obsrv-ui/       Next.js 15 frontend - dashboard with analyze, forensics, monitor, analytics pages
```

## Tech Stack

| Layer | Tech |
|-------|------|
| Backend | Rust, Axum, SQLx, Tokio |
| Streaming | Yellowstone gRPC (Solana) |
| Database | PostgreSQL (Neon) |
| Frontend | Next.js 15, React 19, TypeScript, Tailwind CSS 4, Zustand |
| Alerts | Teloxide (Telegram) |

## Supported Program Decoders

- System Program (transfers, account creation, nonce ops)
- SPL Token / Token-2022 (transfers, approvals, authority changes)
- Compute Budget (unit limits, priority fees)

## Setup

### Backend

```bash
cd obsrv-api
cargo install sqlx-cli --no-default-features --features postgres

# configure .env
DATABASE_URL=postgresql://user:pass@host/dbname?sslmode=require
RPC_URL=https://api.mainnet-beta.solana.com
PORT=3001                          # optional, default 3001
GRPC_ENDPOINT=<yellowstone-url>    # optional
GRPC_X_TOKEN=<token>               # optional
TELEGRAM_BOT_TOKEN=<token>         # optional

sqlx migrate run
cargo run --release
```

### Frontend

```bash
cd obsrv-ui
bun install   # or npm install

# configure .env.local
NEXT_PUBLIC_API_URL=http://localhost:3001

bun dev       # or npm run dev
```

## API

### Analysis

```
POST /analyze              Analyze tx by signature or raw bytes
POST /forensics            Deep forensic inspection
POST /simulate             Simulate raw transaction
POST /nonce/inspect        Inspect durable nonce details
```

### Monitoring

```
POST /monitor/wallet           Add wallet to watchlist
POST /monitor/wallet/remove    Remove wallet
POST /monitor/program          Add program to watchlist
POST /monitor/program/remove   Remove program
GET  /monitor/list             List active monitors
```

### Analytics

```
GET /analytics/wallet?wallet=<ADDRESS>      Wallet stats
GET /analytics/program?program_id=<ID>      Program stats
GET /analytics/programs                     Top programs by volume
GET /analytics/alerts                       Alert history
```

### Streaming

```
GET /stream/transactions       Recent transactions
GET /stream/stats              Overall stats
GET /stream/wallet/history     Wallet tx history
GET /ws                        WebSocket live feed
```

## Database

Migrations in `obsrv-api/migrations/`. Key tables:

- `watched_wallets` / `watched_programs` - per-user monitor subscriptions
- `forensics_history` - analyzed transaction records
- `wallet_analytics` / `program_analytics` - aggregate stats
- `instruction_analytics` / `instruction_daily` - instruction-level breakdowns

## Development

```bash
cargo watch -x run    # backend with hot reload
cargo test            # run tests
cargo clippy          # lint
cargo fmt             # format
```

## License

MIT
