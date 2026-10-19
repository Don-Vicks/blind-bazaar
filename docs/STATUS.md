# Build status (Oct 18, 2026)

What is verified, what is written but unverified, and what does not exist yet. Updated as milestones land. The
README's honesty table is the short form of this file. "Exercised on the testnet service" below means the service
was running against public testnet and the bids were inserted into its database to drive close, pricing and the
pages; no wallet transaction was involved in those checks.

## Added Oct 18 (from the judge, parity and deployment reviews)

- Close is decided by the chain: a bid is late if its block timestamp is after `close_at`; the closer waits for a
  later block and records `close_height`; ties break by mined order. Viewer and spender cannot disagree.
- Distributed key generation (`frost dkg round1|round2|finish|escrow`, file exchange between hosts, no dealer),
  3-party test; one credential per signer host.
- Per-IP rate limit on anonymous endpoints; public `/health` hides balance, mempool and raw errors.
- Honest copy: the browser verifier says "matches what the service reports" and points to the CLI for a chain
  check; custody claims are gated on the instance's signing mode; "who sees what" admits bid counts and the
  Vickrey second price; the widget says the host page sees the connected wallet and that Ethereum calldata shows
  the destination; memo disclosure off by default.
- Seller pages accept the admin token; test networks mint demo seller keys from the dashboard; the bid key is
  shown at bid time and can be pasted on another device.
- Deployment: Dockerfiles copy the vendored crate and workspace packages, `/data` is writable, the web bundle gets
  its public URLs at build time, Caddy gets the host and blocks `/bb/admin*`, same-origin proxy calls pass the
  origin allow-list, `.dockerignore` added. Not yet deployed anywhere.
- Demo script rewritten around what exists (Vickrey lot where the deposit covers the second price, staged Go Dark
  swap, chain-timed close); `docs/evidence/` skeleton with the txid table format and verifier commands.
- CI: GitHub Actions runs the Rust tests and the SDK tests and typechecks the web app on every push.
- Bid cards and my-bids page: a bid card file and a QR link carry the bid key (in the URL fragment) to another
  device; `/bids` lists every bid this browser holds with its standing and what needs attention.
- Opt-in notifications: `POST /auctions/{id}/notify` (bid key + email) and `/seller/notify`; events become mails
  (received, reveal open, won, lost, second chance, refund sent, lot closed, winner paid, paid) through
  `NOTIFY_SMTP_URL`, or are logged without it; addresses purged a week after the lot ends. Not yet sent through a
  real SMTP server.
- Reorg handling: deposits count after `INGEST_CONFIRMATIONS` (default 2); every pass re-checks counted bids and
  settlements against the wallet's view of the chain, sets aside reorged ones (cancelling their refunds),
  reinstates re-mined ones under the same acceptance rules, and flags any that vanish after close for the
  operator. Unit-tested against a stand-in wallet table; no chain reorg exercised.
- Signer ledger check: the viewer exposes `/ledger/payouts` (read-only token); a signer started with
  `--ledger-url` matches every external output of a PCZT against those expected payouts by Orchard receiver and
  exact amount, consuming each once; unknown recipients, wrong amounts and repeats are refused. The spender mirrors
  manual payouts from the viewer so they are in the ledger first. Unit-tested; wired into the regtest FROST run.
- Auditable lots: `auditable: true` creates a per-lot wallet account; the lot's UFVK is published on the result at
  close; deposits are swept to the operator pool at close and after settlement; `bb-auction audit` rescans with the
  key and compares ledger root, price and winners. Exercised on the testnet service: a lot got ZIP 32 account 1,
  the key stayed hidden until close, was published at close, and `bb-auction audit` imported it into a throwaway
  wallet, scanned the public testnet from the birthday and reported MATCH on root, price and winners. That lot had
  no bids. A tampered copy of its result (price and winner changed) made the tool print MISMATCH and exit 2, and unit
  tests cover a suppressed winner, a lowered price, a swapped winner, a planted bid and bad reveals
  (`docs/evidence/audit-tamper.md`); a chain run with wallet bids is still owed. Auditable lots
  need the seed-backed single-process configuration (the viewer must be able to derive the lot account).
- Refund batching: refunds due in the same window (`REFUND_WINDOW_SECS`) from any lot go out as one transaction
  with up to `REFUND_BATCH_MAX` outputs; `bb-auction simulate-refunds` measures timing linkability for naive versus
  batched sending (`docs/evidence/refund-linkability.md`). Unit-tested; no chain run of a batched refund yet.
- Sealed-reveal mode: `reveal_secs` on a lot, `BB1C` commitment memos, commitment root fixed at close, reveal
  endpoint checked against the commitment, unrevealed bids refunded in full, spender mirrors reveals, one-click
  reveal on the status page, sell form defaults to sealed. Unit-tested (36 Rust, 12 SDK tests); no chain run yet.

## Verified working

| Piece | Evidence |
|---|---|
| Rust service syncs public testnet (Ironwood) | `/health` shows lag 0; log shows Sapling/Orchard/Ironwood subtrees |
| Operator wallet creation, dedicated receive address | `data/operator.seed`, address persisted in `meta` |
| Fresh Orchard-only address per auction | `POST /auctions` returns a new `utest1…` each time |
| BB1 memo parse/encode, receipts, ledger hash | 8 Rust tests, 9 SDK tests |
| Server ZIP-321 URI equals SDK URI | byte-for-byte match on a real auction |
| Web: auction list, auction page, bid panel with QR, status page | checked in the browser |
| Go Dark quote routes | live 1Click dry quotes SOL/USDC→`u1`; Maya quotes ETH/USDC→`t1` only since Oct 19 (refuses Orchard-only `u1`; proxy falls back and reports transparent delivery): `docs/evidence/rail-dry-quotes.md` |
| Widget builds (ESM + IIFE) and typechecks | `packages/go-dark/dist` |

## Verified on a real chain (zeckit regtest, `scripts/regtest-e2e.sh`, Oct 13)

Every money path below ran against a local Zebra regtest with lightwalletd and passed:

| Step | Evidence |
|---|---|
| Faucet note detected from compact blocks, balance credited | `operator sees funds` |
| Service builds, proves and broadcasts shielded sends | two 1 ZEC fundings to independent bidder wallets |
| Independent wallets pay a ZIP-321 bid URI with the memo | `b1/b2 paid the deposit with its sealed bid` |
| Memos decrypted after enhancement, bids parsed and validated, receipts published | `both bids ingested from encrypted memos` |
| Close picks the highest bid at or above reserve; ledger hash published | winner 0.3 ZEC over 0.25 |
| Note-opening disclosure built and self-verified | `verified_against_chain: true` |
| Loser refunded shielded to the memo's refund address | `loser refund sent` |
| Settlement memo matched to the winner's nonce | `settlement detected` |
| Seller paid the winning amount | `seller payout sent`, auction `paid` |
| Standalone `--verify-disclosure` reproduces the commitment from the opening | `OK: note opening matches action 3` |
| Late bid (arrived after close) auto-refunded | run 3, bid 2 `late` with refund sent |

Not yet exercised on chain: forfeiture and second chance (unit-tested), Go Dark envelope matching, the
and anything on mainnet. The viewer/spender split ran as two processes on the public testnet on Oct 19 (lot, envelope and manual payout mirrored; the send failed only for lack of funds: `docs/evidence/testnet-two-process.md`).

## Written, not yet exercised against the chain

| Piece | What would prove it |
|---|---|
| Envelope matching for Go Dark bids | a swap whose destination txid matches a received note |
| Go Dark send paths (Phantom USDC/SOL, MetaMask ETH/USDC via Maya router) | one real small swap each on mainnet |
| Shielded delivery through 1Click to a `u1` address | mainnet wet test; docs say transparent-only; `u1` accepted at quote time (dry, Oct 19) |
| Shielded delivery through Maya | not possible today: Maya refuses Orchard-only unified addresses since NU6.3; Ethereum deliveries are transparent and shielded on arrival |
| Memo passthrough in Zodl, Zingo, YWallet from a ZIP-321 QR | one bid from each wallet |

## Added Oct 17 evening (review fixes; unit-tested, exercised on the testnet service)

- Bidder standing: `GET /auctions/{id}/standing?secret=` and a panel on the result and status pages: won/lost/pending,
  balance owed, settlement QR and link, refund state, the seller's delivery note, and (mainnet) Go Dark settlement.
- Bidder secret: the memo nonce is now `sha256(secret)[..16]`; the secret authenticates standing and the delivery note,
  so a disclosed memo cannot be used to read the note. Receipts no longer authenticate anything.
- Blinded ledger leaves (`receipt || bid || nonce`) so public inclusion proofs cannot be brute-forced to amounts.
- Envelopes bound to a fresh address per payload; rail deliveries matched by address only; settlement envelopes;
  spender mirrors envelopes; rail overshoot tolerated; late deliveries refunded.
- FROST signer: one package per spend per session (nonce reuse closed), output commitments and value balance
  verified, non-Orchard components refused, fee capped. Admin token required, constant-time comparisons.
- Second-chance repricing updates the published price; price proof ignores forfeited bids; unmatched settlements
  refunded; seller view hides bidders' addresses and receipts.

## Added Oct 17 (pay button)

- Go Dark packaged as a private pay button for any site: builder with live preview and snippet on `/embed`, CORS on
  the proxy routes with the origin allow-list, hosted bundle rebuilt (`pnpm build:go-dark`), destination optional
  when a fresh-address endpoint is set, and a plain-HTML demo page (`/demo/tip-jar.html`) that mounts the hosted
  script. Verified in the browser up to the wallet step; the mainnet swap itself still needs the wet test.

## Added Oct 17 (formats; unit-tested, exercised against the live service on testnet with seeded bids)

- Vickrey: winner pays the second bid or the reserve; the winning memo is never disclosed; the result API returns
  the runner-up's ledger leaf with its Merkle path (`price_proof`) and the result page verifies it in the browser.
  Second-chance offers reprice to the next bid. 2 close tests.
- Request for quotes: buyer sets a budget and a bond, lowest sealed quote wins, the buyer settles from the
  dashboard (settlement URI on `/seller/lots/{id}`), the winner receives quote plus bond, buyer default returns
  the bond and lapses the request. Disclosures carry `reserve_rule: at_most` so both verifiers read "within
  budget". 2 close tests. `scripts/regtest-e2e.sh FORMAT=vickrey|rfq` written, not yet run (regtest devnet).

## Added Oct 17 (FROST escrow; unit-tested, chain run pending)

- 2-of-3 FROST custody for the spender: `packages/bb-frost` (RedPallas re-randomized FROST, 3 tests),
  `bb-auction frost keygen|show|signer`, escrow account imported from the group viewing key with no spending key,
  PCZT send path (propose → prove → signing session → verify under `rk` → extract → broadcast), signer hosts that
  parse the PCZT themselves and enforce a per-transaction external-value cap, single-use sessions. Four protocol
  tests in `frost.rs` (one signer offline, sub-threshold, policy refusal, session misuse). `/health` reports the
  signing mode; `scripts/regtest-e2e.sh FROST=1` runs the money loop with two signers up and one absent (not yet
  run: needs the regtest devnet). Design in `docs/FROST.md`.

## Added Oct 17

- Seller dashboard: `/dashboard` lists my lots with bids, clearing, settlement and payout summaries; lot detail
  with payouts and timeline; delivery note readable only by a winning receipt, shown on the winner's result page.

- Bidder inclusion proofs: the ledger hash is now a Merkle root; `/auctions/{id}/proof` returns a path; the status
  page verifies it client-side with the bid amount kept local. Shared leaf vector pinned in Rust and SDK tests.

## Added Oct 13 night (unit-tested; chain tests pending)

- In-browser verifier: `packages/bb-verify-wasm` recomputes note commitments and decrypts disclosed memos; the
  verify page per lot loads it from `/wasm` (built by `scripts/build-wasm.sh`)
- Reserve-met proof: optional per-note encryption key in winning disclosures (`DISCLOSE_WINNING_MEMO`), memo
  decryption in both verifiers, assertion in the e2e script
- Refund-latency metric on the result API and page
- How-it-works page with the architecture diagram and the who-sees-what table

## Added Oct 13 evening (unit-tested; chain test of uniform mode pending)

- Uniform-price multi-unit lots: top-N win at the lowest winning bid, per-winner settlement and forfeiture with
  second-chance offers per vacancy, overpayment refunds, multi-output disclosures and a verifier that checks all
- Go Dark: fresh receive address per payment (unified plus transparent receiver), NEAR Intents falls back to the
  transparent receiver only if it rejects the unified address and the widget says so, auto-shield of any transparent
  arrival by the spender, delivery mode and the source-chain leak stated in the widget, `docs/GO-DARK-THREAT-MODEL.md`
- Change pool chosen by NU6.3 activation (Ironwood on mainnet, Orchard before)

## Hardening added Oct 13 (unit-tested, not yet exercised on chain)

- Settlement deadline with forfeiture and second-chance offer to the runner-up
- Payout retries with dead-letter state and admin retry
- Seller API keys (hashed), event log with webhook delivery, reconciliation endpoint, low-balance flag
- Rate limit and origin allow-list on the Go Dark proxy; quote expiry enforced in the widget
- Docker, Compose and Caddy deployment files

## Added Oct 19 (receiver wallet in the browser)

- On `/embed`, "Make a wallet in this browser": 24 BIP 39 words from `crypto.getRandomValues`, the Orchard key by
  the same ZIP 32 path the service uses (in WebAssembly, `packages/bb-verify-wasm`), the transparent receiver by
  BIP 44 in audited curve code, assembled into one unified address; nothing stored or sent. A native test checks
  the derivation against `zcash_keys`, and a phrase generated in the browser gave the same addresses through
  `bb-auction derive`. Import steps for Zodl, Zingo and YWallet with the birthday height.

## Not built yet

- A real swap through Go Dark on mainnet (the widget, proxy and envelope binding are built; no wet test yet).
- Mainnet deployment: VPS, process supervision, HTTPS for the web app, `NEXT_PUBLIC_SITE_URL`, serving `go-dark.iife.js` from a CDN path.
- Refund retry beyond 5 attempts needs a human (the operator is now mailed when that happens, when the float is low, and when the service parks itself: `OPERATOR_EMAIL`).
- Brand palette deferred. (Mobile width and light theme were checked on every page on Oct 18: no horizontal overflow at 375 px, tokens render in both schemes.)
- Video, screenshots, forum/Discord posts, registration, final write-up numbers.

## NU7 on testnet (found Oct 13)

Testnet activated NU7 at height 4,465,026 (around Oct 6). The stable librustzcash crates (0.24 line) only know NU7
as a placeholder, so they cannot parse any testnet transaction since then (`invalid consensus branch id
0x77190ad9`). The service now builds against the Oct 6 pre-release line (`zcash_client_backend 0.25.0-pre.1`,
`zcash_protocol 0.11.0-pre.0`, `orchard 0.16`, `rand 0.10`), which carries the real NU7 branch id and testnet
height. Mainnet NU7 is expected around Nov 5, after the deadline, so the mainnet demo runs on NU6.3 rules with the
same binary. Because judging may run past activation, the service has a freeze: `FREEZE_HEIGHT` (the announced
activation height) parks it at that block (no sync, no closes, no spends; write endpoints answer 503; results,
proofs and standings stay readable; the site shows a "Paused" notice), and lots whose close plus settlement window
would run past the estimated freeze time are refused at creation. `PAUSED=true` parks it by hand. Unit-tested
(`src/freeze.rs`) and exercised on the local service with both settings. Open question: whether lightwalletd 0.5.4 (the public testnet endpoint) includes v6 transaction
outputs in compact blocks; the first received testnet note answers it.

## Known risks in the code

- `ShieldedPool::Orchard` is the fallback change pool in `wallet/send.rs`; verified on a pre-Ironwood regtest, still
  to be confirmed on an Ironwood chain (mainnet).
- The vendored `zcash_client_backend` carries a one-spot patch so servers without Ironwood subtree roots do not
  abort sync; drop it when upstream gates the request on NU6.3 activation.
- Ingest treats every received non-change output as a potential bid; operator funding deposits with no memo stay unprocessed (harmless) and operator self-transfers are excluded.
- In `all` mode without `FROST_ESCROW` the service holds the spending key for refunds and the viewing key in one
  process (development default). Production runs viewer + spender under the FROST escrow.
- The PCZT path (`wallet/pczt_send.rs`) has not yet produced a transaction on a chain; the FROST session and the
  signature verification under `rk` are unit-tested, the wallet-side PCZT build/prove/extract is not.
