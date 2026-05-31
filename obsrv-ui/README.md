# obsrv — UI

Next.js 16 frontend for **obsrv**, a real-time Solana transaction monitor with AI-powered risk analysis.

- **Analyze** — paste a signature or raw bytes, get a verdict, decoded instructions, durable-nonce checks.
- **Forensics** — full execution trace, balance deltas, plain-English narration, program logs.
- **Monitor** — watch wallets and program IDs; live websocket feed of every transaction.
- **Analytics** — wallet / program rollups, network top programs, instruction spikes.

The backend lives in [`../obsrv-api`](https://obsrv.onrender.com) (Rust + Axum + Postgres).

## Quick start

```bash
bun install
bun dev          # http://localhost:3000
```

By default the UI talks to the deployed backend at `https://obsrv.onrender.com`. To point at a local backend instead, edit `.env.local`:

```env
NEXT_PUBLIC_API_URL=http://localhost:3001
```

## Production build

```bash
bun run build
bun run start
```

`.env.production` is committed with `NEXT_PUBLIC_API_URL=https://obsrv.onrender.com`. When deploying to Vercel/Netlify it will be picked up automatically; override it in the host's dashboard if you need a different backend URL.

## Deploy to Vercel

```bash
vercel        # link the project
vercel --prod # ship
```

That's it — no extra config needed.

## Architecture

```
src/
  app/
    layout.tsx        ← root layout, mounts <ObsrvBoot />
    page.tsx          ← sidebar + topbar + page router
    globals.css       ← design tokens (colors, fonts, panel/badge/tab styles)
  components/
    ObsrvBoot.tsx     ← one-time mount: opens websocket, fetches monitor list
    PageAnalyze.tsx
    PageForensics.tsx
    PageMonitor.tsx
    PageAnalytics.tsx
    ErrorAlert.tsx    ← toast container + useAlerts hook
  lib/
    api.ts            ← fetch wrapper + endpoint helpers + connectWs()
    store.ts          ← Zustand store: monitor list, live feed, ws state
    components.tsx    ← shared UI atoms (RiskBadge, ProgramPill, StatCard, …)
```

### Persistent state

`src/lib/store.ts` is a Zustand store mounted once at the app root via `<ObsrvBoot />`. It keeps:

- the monitor list (wallets, programs)
- a split live feed (`walletFeed`, `programFeed`)
- the websocket connection itself

This means **navigating between pages does not reset the live feed**, and the websocket stays connected for the whole session.

### Live feed routing

Every websocket event is dispatched to either the wallet feed, the program feed, or both, depending on which fields the backend sent. The Monitor page's `WALLETS` tab shows the wallet feed; the `PROGRAM IDS` tab shows the program feed.

## Backend contract

The UI expects the API at `NEXT_PUBLIC_API_URL` to expose:

| Method | Path                              | Used by              |
|--------|-----------------------------------|----------------------|
| GET    | `/health`                         | health check         |
| POST   | `/analyze`                        | Analyze tab          |
| POST   | `/simulate`                       | Analyze tab          |
| POST   | `/forensics`                      | Forensics tab        |
| POST   | `/monitor/wallet`                 | Monitor add wallet   |
| POST   | `/monitor/wallet/remove`          | Monitor remove       |
| POST   | `/monitor/program`                | Monitor add program  |
| POST   | `/monitor/program/remove`         | Monitor remove       |
| GET    | `/monitor/list`                   | Monitor list, store  |
| GET    | `/analytics/wallet?wallet=…`      | Analytics → Wallet   |
| GET    | `/analytics/program?program_id=…` | Analytics → Program  |
| GET    | `/analytics/programs?limit=…`     | Network top programs |
| WS     | `/ws`                             | Live feed            |

CORS must allow the deployed frontend origin. For testing, `Access-Control-Allow-Origin: *` works.

## Notes

- The Telegram bot button is intentionally marked **UPCOMING** — the API has the wiring (`telegram_chat_id` field on `/monitor/wallet`) but bot delivery isn't enabled yet.
- The `RPC · helius·mb` / `WSS · 4 subscribed` sidebar indicators were removed; connection state is shown on the live-feed panel itself.
- The top-bar search input was removed — paste signatures directly into the Analyze/Forensics input.

## Tech

- Next.js 16, React 19, TypeScript 5
- Tailwind v4 (only for resets — styles are mostly in `globals.css`)
- Zustand for the cross-page store
- `lucide-react` for icons
