# Deploying a public instance

Two processes serve the site: the Rust auction service (API, wallet sync, closes and payouts) and the Next.js
web app. The files under `deploy/` run both behind Caddy with HTTPS on one small VPS. Nothing here needs more
than 1 vCPU and 2 GB of RAM; the wallet database for a testnet instance is a few hundred megabytes.

## What you need

- A VPS with Docker and Docker Compose (any provider; Ubuntu 24.04 is what the Compose file was written against).
- A domain whose A record points at the VPS. Caddy obtains the certificate itself.
- About ten minutes.

## Steps

1. On the VPS, clone the repository and copy the env examples:
   ```bash
   git clone https://github.com/Don-Vicks/blind-bazaar.git && cd blind-bazaar
   cp deploy/auction.env.example deploy/auction.env
   cp deploy/web.env.example deploy/web.env
   ```
2. Edit `deploy/auction.env`: `NETWORK=test` for the hackathon instance, `LIGHTWALLETD_URL=https://testnet.zec.rocks:443`,
   a long random `ADMIN_TOKEN` and `LEDGER_TOKEN`, `SITE_URL=https://your.domain`, `CORS_ORIGIN=https://your.domain`,
   `TRUST_PROXY=true`, `OPERATOR_EMAIL` and the `NOTIFY_*` values if you want mail. Leave `FREEZE_HEIGHT` unset on
   testnet (NU7 is already active there); on mainnet set it to the announced NU7 height.
3. Edit `deploy/web.env`: `NEXT_PUBLIC_BB_API=https://your.domain/bb`, `NEXT_PUBLIC_SITE_URL=https://your.domain`,
   `GODARK_ALLOWED_ORIGINS=https://your.domain`, `GODARK_TRUST_PROXY=true`.
4. Start it:
   ```bash
   SITE_HOST=your.domain docker compose -f deploy/docker-compose.yml up -d --build
   ```
5. Wait for the first sync (a fresh testnet wallet from a recent birthday takes a minute; the Compose file sets
   `BIRTHDAY_HEIGHT` to a recent height, raise it if the wallet is new):
   ```bash
   curl -s https://your.domain/bb/health
   ```
   `last_sync_at` set and `sync_lag` near 0 means it is live.
6. Fund the operator address shown by `/health` (admin token) from the testnet faucet at fauzec.com (1 TAZ per
   address per day, API: `POST https://fauzec.com/api/v1/claim {"network":"testnet","address":"…"}`). The float
   pays refunds and fees; one TAZ covers a demo day.
7. Mint a seller key and list the first lot from `/sell`, or `POST /bb/sellers/demo` on testnet.

## Operating it

- Admin endpoints are reachable only from the VPS itself (Caddy blocks `/bb/admin*`); use `ssh -L 8787:127.0.0.1:8787`.
- `docker compose logs -f auction` shows every pass: ingest, closes, payouts, notifications, operator alerts.
- Back up `deploy/data/operator.seed`. Losing it loses the float and any deposits in flight.
- To pause before a network upgrade: set `PAUSED=true` (or `FREEZE_HEIGHT`) and restart; reads stay up.

## Alternatives tried

- Railway (CLI logged in on this machine): project creation answered "Deploys have been paused temporarily" on
  Oct 22, so it was not used.
- Vercel for the web app alone is fine (`apps/web`, build command `pnpm --filter web build`, env from
  `deploy/web.env.example`); the service still needs a host that can run a long-lived process with a disk.
