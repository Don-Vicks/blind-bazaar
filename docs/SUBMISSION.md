# ZECATHON submission: every field, ready to paste

Deadline Oct 28, 2026 23:59 UTC. Target: submit Oct 27. Fields marked `TODO` are filled on the day they exist.

## Project

- **Name:** Blind Bazaar
- **One line:** Sealed bids on Zcash, for tenders and auctions: every bid is one shielded payment with the amount
  inside its encrypted memo, the chain closes the round, and the losing bids are never revealed to anyone.
- **Track:** Private Markets (the form allows one).
- **Repository:** https://github.com/Don-Vicks/blind-bazaar (public since Oct 22).
- **Live instance:** the web app and API run on the builder's machine behind an ngrok tunnel; the address changes
  whenever the tunnel restarts (Oct 23: https://6d2b-2605-59c1-1a68-6710-a87f-eaf-df6e-ceb1.ngrok-free.app).
  Before submitting, claim the one free static domain in the ngrok dashboard (Domains) and start the tunnel with
  `ngrok http 3002 --url=<that domain>`, or deploy to a VPS per `docs/DEPLOY.md`. Local recipe: launch config
  `auction` (API on 8787) plus `web-public` (web on 3002 with `NEXT_PUBLIC_BB_API=/bb`, which forwards `/bb/*` to
  the API and blocks admin routes); `scripts/preflight.sh <url>` must be all green.
- **Video:** two minutes (the form's limit): `apps/video/out/demo.mp4`; promo `apps/video/out/promo.mp4` for the socials.
- **Team:** Victor Shallangwa, solo.
- **Write-up:** `docs/WRITEUP.md` (under 800 words). Paste as the description.

## Description for the form (2000 characters max, exactly what the form asks: what it does, why it does not leak, what is real, what is a mock)

```
Blind Bazaar runs sealed-bid rounds on Zcash: tenders, where a buyer posts a job and a budget and the lowest quote wins, and auctions, where the highest bid wins. A bid is one shielded payment to a fresh address, the amount sealed inside the encrypted memo. A viewing key reads bids; the first block after the close time decides the close; losers get their deposit back privately; the winner settles by memo; the result page publishes a note-opening disclosure of the winning payment anyone can check against the chain. Formats: request for quotes, first price, Vickrey (second price proven with a Merkle path), editions. Go Dark is a pay button turning SOL, USDC or ETH into shielded ZEC so people without ZEC can take part.

Why it does not leak: every step on chain is a shielded transaction with no amount, sender or memo visible; receipts are hashes; the ledger root is committed at close; refunds are batched and delayed; deposits come from a fixed menu so an amount never marks a round. Sealed-reveal rounds put only a commitment in the memo, so even the operator cannot read bids early; auditable rounds publish the round's viewing key at close and a CLI tool recomputes the result from the chain and flags a forged one. Spending authority is a 2-of-3 FROST group key: signer hosts check every output against the ledger and co-sign; no single server holds the money.

What is real: three complete rounds on the public Zcash testnet (a tender, an auction, a sealed auditable auction) with bids from an independent wallet, chain close, refund, settlement, payout, verified disclosures, every txid in docs/evidence; five regtest runs across the formats and the FROST escrow, all green; 60+ tests, CI.

What is not: no mainnet round; no bid yet from a mobile wallet (only the bundled CLI wallet); Go Dark quotes are live but no swap has moved real money, and Maya only pays a transparent receiver since Ironwood, which the widget says. On classic rounds the operator reads bids while a round is open.
```

## What a judge can verify in five minutes

1. Public testnet lot `jck1vba7`: two shielded bids from an independent wallet, closed by the chain, loser
   refunded, winner settled, seller paid. Every txid in `docs/evidence/testnet-jck1vba7/txids.md`, checkable on
   any testnet explorer; the result page shows the proof and the note-opening disclosure.
2. Public testnet lot `ejp8jz0s`: sealed-reveal and auditable; the audit from the published key matches the
   result and a tampered copy is caught (`docs/evidence/testnet-ejp8jz0s/`).
3. The disclosure verifier against a public lightwalletd:
   `bb-auction --network test --lightwalletd-url https://testnet.zec.rocks:443 --verify-disclosure disclosure.json`.
4. The audit tool catching a forged result: `docs/evidence/audit-tamper.md` (MISMATCH, exit 2).
5. The regtest money loop across formats and the FROST escrow: `docs/evidence/regtest/`.
6. `cargo test -p bb-frost -p bb-auction -p bb-verify-wasm` and the CI badge: everything green.

## Honesty, in one paragraph (paste if asked "what does not work")

On classic lots the operator reads bids while a lot is open and could shill; sealed-reveal lots remove that,
auditable lots let anyone recompute the result, and the receipts, ledger root and disclosure bound what a
dishonest operator can get away with. The FROST escrow has signed on a local regtest chain, not yet on a public
one. Go Dark's rails are mainnet-only and no swap has moved real money; Maya refuses Orchard-only unified
addresses since Ironwood, so the Ethereum path delivers transparently and is shielded on arrival, and the widget
says so. Only the bundled command-line wallet and (TODO) one mobile wallet have placed a bid.

## Questions to send the organisers today

1. May one project be entered in more than one track?
2. Is a repository that goes public on submission day acceptable, or must it be public earlier?
3. Is there a video length limit beyond "about three minutes"?

## Day plan to the deadline

| Day | Do |
|---|---|
| Oct 22 | Regtest batch, public testnet lot end to end, repo public, deploy runbook. |
| Oct 23 | Deploy to a VPS (needs a host and a domain), stage three demo lots, Zodl testnet bid if a testnet build exists. |
| Oct 24 | Dry run of the demo script on the deployed instance; backup recording. |
| Oct 25 | Code freeze except fixes. Final numbers into the write-up and README. |
| Oct 26 | Record the video twice, screenshots, fill this document. |
| Oct 27 | Submit. |
