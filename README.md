# Blind Bazaar + Go Dark

**Blind Bazaar** is a sealed-bid auction house on Zcash. Every bid is a shielded transaction whose encrypted memo
carries the bid. No smart contract: the chain is the mailbox, the viewing key is the inbox, and one operator wallet
refunds losers and settles the winner. Losing bids are never revealed; the winning transaction is provable.

**Go Dark** is the front door: an embeddable button that turns SOL, USDC or ETH from Phantom or MetaMask into
shielded ZEC at a unified address, and can carry a sealed bid along with it.

Built for ZECATHON 2026. MIT licensed.

## What is private, what is real, what is not

| Claim | Status | How to check |
|---|---|---|
| Bids are private. Only the operator's viewing key can read a bid, and only after it is mined; other bidders and the seller never see losing bids. | Real, with one stated limit: the operator reads bids while a lot is open. | `docs/PRIVACY.md`; memo layout in `docs/MEMO_SPEC.md` |
| The result is provable. A note-opening disclosure of the winning payment is published and recomputed in the browser and by a CLI. | Real on a local regtest chain and against the testnet service. No mainnet lot yet. | `/a/{id}/verify`, `bb-auction --verify-disclosure` |
| The bid ledger cannot be altered after close. A Merkle root over every valid bid is published before any refund; each bidder verifies their own leaf; leaves are blinded by the bidder's nonce. | Real. | `/a/{id}/status`, `GET /auctions/{id}/proof` |
| Refunds are private. Losing deposits return as shielded payments after a random delay. | Real on regtest; measured refund latency on the result page. | `scripts/regtest-e2e.sh` |
| No single server holds the money. The spend authority is a 2-of-3 FROST group key; signer hosts verify every output and co-sign. | Built and unit-tested. Not yet exercised on a chain. Trusted-dealer keygen; DKG not built. | `docs/FROST.md`, `cargo test -p bb-auction frost` |
| Four formats: first price, uniform-price editions, Vickrey (winning bid stays sealed, second price proven from the ledger), request for quotes. | Real; Vickrey and RFQ exercised on the testnet service with seeded bids. | `docs/STATUS.md` |
| Go Dark: pay or bid from Solana or Ethereum, land in shielded ZEC. | Built on live mainnet rails (Maya, NEAR Intents); quotes are real; no completed swap yet. The rail sees the payer's source address. | `docs/GO-DARK-THREAT-MODEL.md`, `/embed` |
| Bids are hidden from the operator too (sealed-reveal). A lot created with `reveal_secs` takes a commitment `sha256(auction|bid|salt)` in the memo instead of the amount; the commitment root is fixed at close; bidders reveal in the window; unrevealed bids are refunded in full. | Built and unit-tested (Oct 18). The reveal is one click on the status page with the bid key; it has not yet run on a chain. | `docs/PRIVACY.md`, `POST /auctions/{id}/reveals` |
| In-browser Zcash wallet for recipients without one. | Not built. | |

## Layout

| Path | What |
|---|---|
| `services/auction` | Rust service: wallet sync (`zcash_client_sqlite`), memo ingest, close, refunds, HTTP API |
| `apps/web` | Next.js app: auctions, bid QR, receipts, results, Go Dark demo and rail proxies |
| `packages/bb-sdk` | TypeScript: BB1 memo format, ZIP-321 URIs, receipts |
| `packages/go-dark` | The widget (`<GoDark/>` React component and `GoDark.mount` script bundle) |
| `docs/MEMO_SPEC.md` | The memo format |
| `docs/PRIVACY.md` | Who sees what, known limits, rail findings |
| `docs/GO-DARK-THREAT-MODEL.md` | What each hop reveals when entering through Go Dark, and the mitigations |
| `docs/LANDSCAPE.md` | Arcium, comparable hackathon winners, the organiser's own auction, what we change |

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

Sellers: create a key once with `POST /admin/sellers`, then use the **Sell** link. The dashboard shows each lot's
bids, clearing price, settlement progress and payouts, and lets the seller post a delivery note that only a winning
receipt can read.

## Test every money path locally

With Docker running: `cargo install zeckit && zeckit up --backend lwd`, then `scripts/regtest-e2e.sh`. It funds the
operator from the local faucet, has the operator fund two independent bidder wallets, lists an auction, places two
sealed bids, waits for close, and checks the winner, the disclosure proof, the loser's refund, the winner's settlement,
the seller payout and the standalone verifier. About 15 minutes end to end.

## Pay button

The Go Dark widget doubles as a private pay button for any site: tips, checkout, donations. `/embed` builds the
snippet with a live preview; `/demo/tip-jar.html` is a plain HTML page using it through the hosted
`go-dark.iife.js`. The proxy routes under `/api/godark/*` send CORS headers for origins in
`GODARK_ALLOWED_ORIGINS` (any origin when unset). Rebuild the hosted bundle with `pnpm build:go-dark`. What the
rail sees is stated on the page and in `docs/GO-DARK-THREAT-MODEL.md`: private on the Zcash side and from the
hosting site, not from the swap service.

## Lot formats

- **Single lot, first price**: the highest bid at or above reserve wins and pays its own bid.
- **Edition, uniform price** (`units` > 1): the top N bids win and every winner pays the lowest winning bid. Each
  winner settles separately; a winner who does not pay forfeits the deposit and the next bidder is offered the unit.
- **Vickrey, second price** (`format: vickrey`): the highest bid wins and pays the second-highest bid, or the
  reserve if there is none. The winning bid is never disclosed, not even its memo; the price is proven by
  publishing the runner-up's ledger leaf (receipt and amount) with its Merkle path to the ledger commitment, which
  the result page verifies in the browser. The runner-up stays anonymous.
- **Request for quotes** (`format: rfq`): the lot creator is a buyer. `reserve_zats` is the budget, `deposit_zats`
  the bond every quoter posts. The lowest quote within budget wins; the buyer pays it from the dashboard with a
  settlement memo and the winner receives the quote plus their bond. Losing bonds are refunded. If the buyer does
  not pay in time the bond is returned and the request lapses. Only the winning price is ever revealed.

`FORMAT=vickrey` or `FORMAT=rfq` runs the regtest end-to-end script in that format.

## Sealed-reveal lots

Create a lot with `reveal_secs` (60 to 604800) and bids carry `BB1C|<auction>|<commitment>|<refund>|<nonce>` with
`commitment = sha256("<auction>|<bid_zats>|<salt>")`. The operator's viewing key sees deposits arrive and nothing
about amounts. At close the lot enters `revealing`: a Merkle root over `(receipt, commitment)` is published, then
bidders reveal `(bid_zats, salt)` with their bid key; each reveal is checked against the commitment. When the window
ends, unrevealed commitments are refunded in full and cannot win, and the lot closes over the revealed bids with the
usual ledger root, pricing and proofs. Any format except request-for-quotes can be sealed.

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
| POST | `/auctions` | `x-admin-token` or `x-seller-key` header |
| GET | `/auctions/{id}/proof?receipt=` | Merkle inclusion proof of one receipt in the ledger committed at close |
| GET | `/auctions/{id}/events` | public event log (created, bid received, closed, refund sent, settled, paid, forfeited) |
| GET | `/seller/me`, `/seller/lots`, `/seller/lots/{id}` | seller dashboard data (`x-seller-key`): lots with money summary, events, payouts |
| POST | `/seller/lots/{id}/fulfilment` | delivery note for the winner |
| GET | `/auctions/{id}/fulfilment?receipt=` | the delivery note, only with a winning receipt |
| POST | `/admin/sellers` | create a seller key (returned once, stored hashed) |
| GET | `/admin/reconcile` | deposits and settlements in, payouts out, expected vs wallet balance |
| POST | `/admin/payouts/{id}/retry` | requeue a dead or failed payout |
| GET | `/auctions`, `/auctions/{id}` | |
| GET | `/auctions/{id}/receipts` | receipts of received bids |
| GET | `/auctions/{id}/bid-uri?bid_zats=&refund_address=&nonce=` | server-side ZIP-321 builder |
| POST | `/auctions/{id}/envelopes` | register a sealed payload (`BB1` bid or `BB1S` settlement) for a rail delivery; returns the fresh address to deliver to |
| GET | `/auctions/{id}/standing?secret=` | the bidder's own view: outcome, balance owed, settlement URI, refund, delivery note |
| POST | `/godark/receive-address` | fresh unified address (with transparent receiver) for one Go Dark payment |
| GET | `/auctions/{id}/result` | winner, ledger hash, refunds, disclosure |

## Verify a result yourself

In the browser: every result page links to `/a/<id>/verify`, which recomputes each winning note's commitment with
the Orchard code compiled to WebAssembly and, when a memo key is disclosed, decrypts that one memo locally. Build the
module with `scripts/build-wasm.sh` (needs `rustup target add wasm32-unknown-unknown` and `cargo install
wasm-bindgen-cli`); it lands in `apps/web/public/wasm`.

From the command line:

Every closed auction publishes a note-opening disclosure of the winning output. Anyone can check it against the
chain with only the service binary and a public lightwalletd, no wallet or keys:

```bash
curl -s http://127.0.0.1:8787/auctions/<id>/result | jq .disclosure > disclosure.json
cargo run --release -- --verify-disclosure disclosure.json --lightwalletd-url https://testnet.zec.rocks:443
```

It rebuilds the Orchard note from the disclosed opening (recipient, value, rho, rseed), computes its commitment and
compares it with the `cmx` of the named action in the transaction. With `DISCLOSE_WINNING_MEMO=true` the disclosure
also carries that note's encryption key; the verifier then decrypts the memo from the transaction and reports whether
the bid met the reserve. Losing bids are never disclosed.

## Operating it

- Run two processes for custody separation: `ROLE=viewer OPERATOR_UFVK=uview…` (no seed on that host)
  and, elsewhere, `ROLE=spender VIEWER_URL=https://viewer ADMIN_TOKEN=…` with the seed. Get the UFVK with
  `bb-auction derive --seed-file operator.seed --ufvk`. `docs/PRIVACY.md` explains what each role can do.
- Or run the spender without any spending key: `bb-auction frost keygen --out ./frost` makes a 2-of-3 FROST
  escrow; signer hosts run `bb-auction frost signer --share signer-N.json --escrow escrow.json`; the spender takes
  `FROST_ESCROW` and `FROST_SIGNERS`, and every payout is a PCZT the signers inspect and co-sign. `docs/FROST.md`.

- A winner who does not pay within `SETTLEMENT_WINDOW_SECS` (default 24h) forfeits the deposit to the seller; the
  runner-up is offered the lot if their refund has not been sent yet, otherwise the auction ends unsettled.
- Payouts retry with backoff and are parked as `dead` after 5 attempts; `/health` reports `dead_payouts` and a
  `low_balance` flag when the wallet cannot cover queued payouts plus `FEE_RESERVE_ZATS`.
- Set `WEBHOOK_URL` to receive every event as a JSON POST (retried up to 8 times).
- Set `CORS_ORIGIN` in production; the default allows any origin for development.
- See `deploy/` for Docker images, a Compose file with HTTPS, and env templates.

## Zcash primitives used

Ironwood-pool shielded transactions to Orchard receivers, 512-byte encrypted memos, unified full viewing keys
(view-only account, no spend authority over bids beyond the deposit), fresh diversified unified addresses per
auction, ZIP-321 payment URIs, and a ZIP 311-style note-opening disclosure of the winning output.

## Known limits

The operator can read bids while an auction is open. See `docs/PRIVACY.md` for mitigations and the v2 plan.
Cross-chain bids carry the bid in an envelope posted to the service rather than in the on-chain memo, because no
swap rail can attach a memo to the ZEC it delivers. The trust model is the same either way.
