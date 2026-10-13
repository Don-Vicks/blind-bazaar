# Build status (Oct 13, 2026)

What is verified, what is written but unverified, and what does not exist yet. Updated as milestones land.

## Verified working

| Piece | Evidence |
|---|---|
| Rust service syncs public testnet (Ironwood) | `/health` shows lag 0; log shows Sapling/Orchard/Ironwood subtrees |
| Operator wallet creation, dedicated receive address | `data/operator.seed`, address persisted in `meta` |
| Fresh Orchard-only address per auction | `POST /auctions` returns a new `utest1…` each time |
| BB1 memo parse/encode, receipts, ledger hash | 8 Rust tests, 9 SDK tests |
| Server ZIP-321 URI equals SDK URI | byte-for-byte match on a real auction |
| Web: auction list, auction page, bid panel with QR, status page | checked in the browser |
| Go Dark quote routes | live Maya quote ETH→`u1`, live 1Click dry quote USDC→`u1` |
| Widget builds (ESM + IIFE) and typechecks | `packages/go-dark/dist` |

## Written, not yet exercised against the chain

| Piece | What would prove it |
|---|---|
| Memo ingest from real received notes (`v_tx_outputs` query, txid byte order, `to_address`) | a bid landing in `bids` table |
| Receipt publication | `/auctions/{id}/receipts` listing a real receipt |
| Close, winner selection, ledger hash | a short auction closing with two bids |
| Refunds via `propose_transfer` + `create_proposed_transactions` (fallback change pool, fees, confirmations) | a refund txid in `payouts` and the bidder wallet seeing it |
| Settlement memo `BB1S` matching and seller payout | winner pays balance, seller receives |
| Disclosure build (`orchard_received_notes` join, note version, cmx match) and `--verify-disclosure` | `verified_against_chain: true` and CLI printing OK |
| Envelope matching for Go Dark bids | a swap whose destination txid matches a received note |
| Go Dark send paths (Phantom USDC/SOL, MetaMask ETH/USDC via Maya router) | one real small swap each on mainnet |
| Shielded delivery through 1Click to a `u1` address | mainnet wet test; docs say transparent-only |
| Memo passthrough in Zodl, Zingo, YWallet from a ZIP-321 QR | one bid from each wallet |

## Not built yet

- Seller listing UI (`/sell`); auctions are created with curl and the admin token.
- In-browser receiver wallet generation for Go Dark (24-word seed + unified address, Zodl/Zingo import).
- Transparent-receiver fallback with auto-shield if 1Click cannot deliver to `u1` from Solana.
- Rotation of per-payment addresses inside the widget (currently one `to` address per mount).
- Bidder-side inclusion proof against the ledger hash (only the hash is published).
- Mainnet deployment: VPS, process supervision, HTTPS for the web app, `NEXT_PUBLIC_SITE_URL`, serving `go-dark.iife.js` from a CDN path.
- Operator balance alerts; refund retry beyond 5 attempts needs a human.
- Light theme check and mobile width pass on every page; brand palette deferred.
- Video, screenshots, forum/Discord posts, registration, final write-up numbers.

## NU7 on testnet (found Oct 13)

Testnet activated NU7 at height 4,465,026 (around Oct 6). The stable librustzcash crates (0.24 line) only know NU7
as a placeholder, so they cannot parse any testnet transaction since then (`invalid consensus branch id
0x77190ad9`). The service now builds against the Oct 6 pre-release line (`zcash_client_backend 0.25.0-pre.1`,
`zcash_protocol 0.11.0-pre.0`, `orchard 0.16`, `rand 0.10`), which carries the real NU7 branch id and testnet
height. Mainnet NU7 is expected around Nov 5, after the deadline, so the mainnet demo runs on NU6.3 rules with the
same binary. Open question: whether lightwalletd 0.5.4 (the public testnet endpoint) includes v6 transaction
outputs in compact blocks; the first received testnet note answers it.

## Known risks in the code

- `ShieldedPool::Orchard` is passed as the fallback change pool in `wallet/send.rs`; post-Ironwood the strategy should redirect change, but this is unverified until the first send.
- Ingest treats every received non-change output as a potential bid; operator funding deposits with no memo stay unprocessed (harmless) and operator self-transfers are excluded.
- The service holds the spending key for refunds and the viewing key in one process (documented limitation).
