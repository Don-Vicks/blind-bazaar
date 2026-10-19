# Viewer and spender as two processes, on the public testnet (Oct 19, 2026)

The split is the custody story: the process that serves the API and reads bids holds only the viewing key; the
process that signs payouts holds the seed (or, with FROST, only the escrow's viewing key and a coordinator
role) and takes its orders from the viewer's ledger. Until today it had only compiled and passed unit tests.

## Setup

Two `bb-auction` processes on one machine, separate data directories, both synced from testnet birthday
4,515,600 through `https://testnet.zec.rocks:443` (launch configs `viewer` and `spender` in `.claude/launch.json`):

| | viewer | spender |
|---|---|---|
| `ROLE` | `viewer` | `spender` |
| key | `OPERATOR_UFVK` (from `bb-auction derive --ufvk`) | `OPERATOR_SEED_FILE` |
| `/health.signing` | `null` | `"seed"` |
| talks to | lightwalletd | lightwalletd and the viewer (`VIEWER_URL`) |
| bind | 127.0.0.1:8787 | 127.0.0.1:8788 |

## What happened

1. A lot was created on the viewer (`POST /auctions`, admin token). The viewer derived its address from the
   viewing key alone: `utest13xsl0mjl…`. `bb-auction derive --check` on the seed confirms the spender's key owns
   that address (diversifier index 3f29c312…), so deposits to it are spendable by the spender without the two
   processes ever sharing a secret.
2. A Go Dark style envelope was posted to the viewer for that lot (`POST /auctions/{id}/envelopes`); the viewer
   bound it to a fresh address `utest1esyw8sj…`.
3. A manual payout of 10,000 zats was queued on the viewer (`POST /admin/payouts`). The viewer's
   `GET /ledger/payouts?kind=manual` (read-only `x-ledger-token`) listed it; the admin token header was refused
   for that endpoint when it carried the ledger token, as intended.
4. On its next pass the spender logged:
   ```
   mirrored auction 1ay3qpp1 (two-process run)
   mirrored 1 envelope(s)
   mirrored manual payouts up to id 1
   sending 1 payout(s) [1] totalling 10000 zats
   payout batch [1] failed: propose_transfer: InsufficientFunds { available: Zatoshis(0), required: Zatoshis(20000) }
   ```
   Its own database then held the lot (same address as the viewer's), the envelope and the payout, marked
   `failed` after one attempt with that error, which is the correct outcome for a wallet holding no testnet ZEC.

## What this proves and what it does not

- Proves: a viewer with no spending key can run the public API and derive lot addresses the spender can spend
  from; the spender adopts lots, envelopes and operator payouts from the viewer and acts on them; the ledger
  endpoint is readable with the ledger token and not with a wrong one.
- Does not prove: a payout actually leaving the spender and being reported back to the viewer
  (`POST /admin/payouts/sync`), because the operator wallet holds no testnet ZEC. That is the next step once the
  wallet is funded, and the same run with `FROST_ESCROW` set is the chain run the FROST docs still owe.
- Note: each process picked its own operator receive address on first start (the viewer's is the public one);
  both are owned by the same key.
