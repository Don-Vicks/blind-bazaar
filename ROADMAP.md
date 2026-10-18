# Roadmap: from hackathon demo to product

Status key: **done** · **in progress** · **planned**. Dates are targets.

## Phase 0 — Judge-grade demo (submissions due Oct 28, 2026)

- done: service, web app, SDK, widget, memo spec, privacy model, disclosure proof, NU7-ready crates
- done on regtest (Oct 13): full funded loop (fund → bid → close → refund → settle → payout → proof verifies); testnet and mainnet runs still to do
- planned: three real wallets (Zodl, Zingo, YWallet) place bids from the QR; one Go Dark swap lands shielded
  from Phantom and one from MetaMask; mainnet dry run under 2 ZEC; video, write-up, screenshots; registration
- done: uniform-price multi-unit mode (the zkSNARKs and Crafts format) with per-winner settlement and forfeiture
- done: Go Dark as the bid entry path with a written threat model, fresh address per payment, transparent fallback
  with auto-shield, and the source-chain leak stated in the widget
- done: public verify page (browser-side WebAssembly), refund-latency metric, reserve-met proof via per-note key
  disclosure, how-it-works page
- planned: one completed mainnet lot, linked from the verify page
- stretch: Arcium MPC mode for winner selection (bids encrypted to an MXE, Vickrey or uniform clearing in MPC,
  result sealed to the seller, uniform shielded ZEC deposits carrying the Solana receipt hash)

## Phase 1 — Hardening (Nov 2026)

Make the money paths safe before anyone who is not us uses them.

- done: settlement deadline: a winner who does not pay within the window forfeits the deposit to the seller and
  the runner-up gets a second-chance offer if their refund has not left yet
- done: payout queue with bounded retries, dead-letter state and admin retry; cancelled refunds on second chance
- done: reconciliation endpoint (expected vs actual wallet balance) and low-balance warning in `/health`
- done: seller API keys (hashed at rest) instead of one shared admin token for listing
- done: event log with webhook delivery (bid received, closed, refund sent, settled, paid, forfeited)
- done: rate limit and origin allow-list on the Go Dark quote proxy; quote expiry enforced in the widget
- done: Docker images and a Compose file with HTTPS (Caddy) for a single-VPS deployment
- done: integration test against a regtest chain (zeckit) covering every money path (`scripts/regtest-e2e.sh`)
- done: confirmation gate on ingest and per-pass reorg re-checks with reinstatement (unit-tested; not yet exercised on a chain reorg)
- planned: checkpoint backups of both databases
- planned: structured audit log with no addresses; metrics (sync lag, queue depth, balance) to Prometheus
- planned: pin to a released librustzcash line once NU7 ships on mainnet (Nov 5 target)

## Phase 2 — Custody split (Dec 2026)

Remove the single operator who can both read bids and move funds.

- done (needs chain test): two processes with distinct keys: a **viewer** holding only the UFVK (ingests bids,
  publishes receipts, decides winners) and a **spender** holding the spending key that mirrors auction
  definitions immutably, recomputes outcomes from its own wallet view, and only executes payouts its own ledger
  derives
- done (unit-tested; chain run pending): FROST 2-of-3 threshold signing for the spender: the escrow account has no
  spending key, every payout is a PCZT co-signed by independent signer hosts that inspect it first (`docs/FROST.md`)
- done: distributed key generation (`frost dkg`, file exchange between hosts); per-signer credentials; signer verifies every output commitment, value balance and fee
- done: signers match every output against the viewer's ledger of expected payouts, each usable once
- planned: rolling spend caps per signer
- done: bidder-side inclusion proofs: Merkle commitment of the bid ledger at close, per-receipt proof verified in the browser
- planned: encrypted-to-auction-key bid envelopes for cross-chain entries (rail and relay see ciphertext only)
- done: refund batching across lots per window, with a measured timing-linkability table (`docs/evidence/refund-linkability.md`)
- done: auditable lots (per-lot wallet account, viewing key published at close, `bb-auction audit` recomputes and compares)
- planned: denomination splitting of refunds across lots

## Phase 3 — Product (Q1 2027)

- done: seller dashboard (lots, money, payouts, events) and a winner-gated delivery note
- planned: seller reputation; escrow for physical goods
- planned: winner notifications (email, push, wallet memo) and seller dashboard
- done: Vickrey (second price, winning bid sealed, price proven from the ledger) and private request-for-quote
- done: sealed-reveal mode (commitment in the memo, reveal window, unrevealed bids refunded) so the operator cannot read bids before close
- planned: Dutch format; batch markets (private order matching) on the same rails
- planned: Go Dark receiver onboarding (in-browser 24-word wallet, Zodl/Zingo import), per-payment address
  rotation, transparent-receiver fallback with auto-shield, hosted widget CDN and npm release
- planned: mobile wallet certification matrix for ZIP-321 memo handling
- planned: light theme, brand, accessibility audit, localisation

## Phase 4 — Operations and compliance (ongoing)

- planned: legal review of money-transmission exposure per jurisdiction; terms of service; sanctions screening
  policy for the rail proxy; incident response and key-rotation runbooks
- planned: bug bounty and an external audit of the service and widget before any mainnet volume

See `docs/STATUS.md` for what is verified today and `docs/PRIVACY.md` for the trust model.
