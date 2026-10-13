# Blind Bazaar + Go Dark

**Blind Bazaar** is a sealed-bid auction house on Zcash. Every bid is a shielded transaction whose encrypted memo
carries the bid. No smart contract: the chain is the mailbox, the viewing key is the inbox, and one operator wallet
refunds losers and settles the winner. Losing bids are never revealed; the winning transaction is provable.

**Go Dark** is the front door: an embeddable button that turns SOL, USDC or ETH from Phantom or MetaMask into
shielded ZEC at a unified address, and can carry a sealed bid along with it.

Built for ZECATHON 2026. MIT licensed.

## Layout

| Path | What |
|---|---|
| `services/auction` | Rust service: wallet sync (`zcash_client_sqlite`), memo ingest, close, refunds, HTTP API |
| `apps/web` | Next.js app: auctions, bid QR, receipts, results, Go Dark demo and rail proxies |
| `packages/bb-sdk` | TypeScript: BB1 memo format, ZIP-321 URIs, receipts |
| `packages/go-dark` | The widget (`<GoDark/>` React component and `GoDark.mount` script bundle) |
| `docs/MEMO_SPEC.md` | The memo format |
| `docs/PRIVACY.md` | Who sees what, known limits, rail findings |

## Run it locally (testnet)

Prerequisites: Rust 1.88+, Node 22+, pnpm 10.

```bash
pnpm install
pnpm -r build                      # builds the SDK and widget
cp services/auction/.env.example services/auction/.env
cp apps/web/.env.example apps/web/.env.local
```

Start the auction service (first run creates `data/operator.seed` and syncs from the chain tip):

```bash
cd services/auction && set -a && source .env && set +a && cargo run --release
```

Start the web app:

```bash
pnpm dev
```

Open http://localhost:3000. The header badge shows sync state. `GET http://127.0.0.1:8787/health` shows the
operator address; fund it with testnet ZEC (https://fauzec.com) so it can send refunds.

Create an auction (replace the seller address with a unified address you control):

```bash
curl -X POST http://127.0.0.1:8787/auctions -H 'content-type: application/json' -H 'x-admin-token: change-me' \
  -d '{"title":"1-of-1 print","reserve_zats":2000000,"deposit_zats":1000000,"seller_address":"utest1...","duration_secs":3600}'
```

Bid from Zodl (formerly Zashi), Zingo or YWallet by scanning the QR on the auction page.

## How a bid works

1. The seller lists a lot: title, reserve, close time, deposit. The service derives a fresh Orchard-only unified
   address for the auction from the operator wallet.
2. The bidder's browser builds a ZIP-321 URI: amount = deposit, memo = `BB1|<auction>|<bid>|<refund address>|<nonce>`.
3. The bidder pays from any ZIP-321 wallet, or from Solana/Ethereum through Go Dark. The memo is encrypted on chain.
4. The service syncs with lightwalletd, fetches full transactions, decrypts memos and records bids. It publishes
   `sha256(nonce)` receipts so a bidder can confirm "bid received" without revealing anything.
5. At close the highest valid bid at or above reserve wins. The service publishes a hash of the bid ledger, refunds
   losers with random delays, and the winner pays `bid - deposit` with memo `BB1S|<auction>|<nonce>`. Then the
   seller is paid.
6. The result page shows the winning amount, the winning txid and a note-opening disclosure of that output only.

## API

| Method | Path | Notes |
|---|---|---|
| GET | `/health` | sync lag, operator address, balance |
| POST | `/auctions` | `x-admin-token` header |
| GET | `/auctions`, `/auctions/{id}` | |
| GET | `/auctions/{id}/receipts` | receipts of received bids |
| GET | `/auctions/{id}/bid-uri?bid_zats=&refund_address=&nonce=` | server-side ZIP-321 builder |
| POST | `/auctions/{id}/envelopes` | Go Dark bid envelope `{deposit_address, rail, payload, destination_txid?}` |
| GET | `/auctions/{id}/result` | winner, ledger hash, refunds, disclosure |

## Zcash primitives used

Ironwood-pool shielded transactions to Orchard receivers, 512-byte encrypted memos, unified full viewing keys
(view-only account, no spend authority over bids beyond the deposit), fresh diversified unified addresses per
auction, ZIP-321 payment URIs, and a ZIP 311-style note-opening disclosure of the winning output.

## Known limits

The operator can read bids while an auction is open. See `docs/PRIVACY.md` for mitigations and the v2 plan.
Cross-chain bids carry the bid in an envelope posted to the service rather than in the on-chain memo, because no
swap rail can attach a memo to the ZEC it delivers. The trust model is the same either way.
