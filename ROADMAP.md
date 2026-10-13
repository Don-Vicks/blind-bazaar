# Roadmap: from hackathon demo to product

Status key: **done** · **in progress** · **planned**. Dates are targets.

## Phase 0 — Judge-grade demo (by Oct 29, 2026)

- done: service, web app, SDK, widget, memo spec, privacy model, disclosure proof, NU7-ready crates
- in progress: first funded loop on testnet (bid → close → refund → settle → payout → proof verifies)
- planned: three real wallets (Zodl, Zingo, YWallet) place bids from the QR; one Go Dark swap lands shielded
  from Phantom and one from MetaMask; mainnet dry run under 2 ZEC; video, write-up, screenshots; registration

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
- planned: integration tests against a regtest chain (zeckit) covering every money path
- planned: wallet rescan and reorg handling exercised deliberately; checkpoint backups of both databases
- planned: structured audit log with no addresses; metrics (sync lag, queue depth, balance) to Prometheus
- planned: pin to a released librustzcash line once NU7 ships on mainnet (Nov 5 target)

## Phase 2 — Custody split (Dec 2026)

Remove the single operator who can both read bids and move funds.

- planned: two processes with distinct keys: a **viewer** holding only the UFVK (ingests bids, publishes
  receipts, decides winners) and a **spender** holding only the spending key (executes payouts it can verify
  against signed auction rules and the on-chain ledger commitment)
- planned: FROST threshold signing for the spender so refunds and payouts need k-of-n operators
- planned: bidder-side inclusion proofs: Merkle commitment of the bid ledger at close, per-receipt proof
- planned: encrypted-to-auction-key bid envelopes for cross-chain entries (rail and relay see ciphertext only)
- planned: refund batching and timing analysis with documented k-anonymity targets

## Phase 3 — Product (Q1 2027)

- planned: seller onboarding, reputation and listing pages; item delivery and escrow flows for digital goods
- planned: winner notifications (email, push, wallet memo) and seller dashboard
- planned: auction formats: Vickrey (second price), Dutch, OTC request-for-quote
- planned: Go Dark receiver onboarding (in-browser 24-word wallet, Zodl/Zingo import), per-payment address
  rotation, transparent-receiver fallback with auto-shield, hosted widget CDN and npm release
- planned: mobile wallet certification matrix for ZIP-321 memo handling
- planned: light theme, brand, accessibility audit, localisation

## Phase 4 — Operations and compliance (ongoing)

- planned: legal review of money-transmission exposure per jurisdiction; terms of service; sanctions screening
  policy for the rail proxy; incident response and key-rotation runbooks
- planned: bug bounty and an external audit of the service and widget before any mainnet volume

See `docs/STATUS.md` for what is verified today and `docs/PRIVACY.md` for the trust model.
