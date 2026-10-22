# ZECATHON submission: every field, ready to paste

Deadline Oct 28, 2026 23:59 UTC. Target: submit Oct 27. Fields marked `TODO` are filled on the day they exist.

## Project

- **Name:** Blind Bazaar + Go Dark
- **One line:** Sealed-bid auctions where every bid is a shielded Zcash payment with the bid inside its encrypted
  memo, plus a pay button that lets anyone on Solana or Ethereum pay or bid in shielded ZEC.
- **Tracks:** Private markets (primary). Also cross-chain (Go Dark) and core and tooling (bid SDK, audit and
  disclosure verifiers). Ask the organisers whether one entry may list several tracks before submitting.
- **Repository:** https://github.com/Don-Vicks/blind-bazaar (make it public before submitting: Settings →
  General → Danger zone → Change visibility; the secret scan on Oct 22 found nothing tracked that should not be).
- **Live instance:** TODO (see `docs/DEPLOY.md`; the demo runs on Zcash testnet).
- **Video:** TODO, three minutes, script in `docs/DEMO_SCRIPT.md`.
- **Team:** Victor Shallangwa, solo.
- **Write-up:** `docs/WRITEUP.md` (under 800 words). Paste as the description.

## What a judge can verify in five minutes

1. Public testnet lot `jck1vba7`: two shielded bids from an independent wallet, closed by the chain, loser
   refunded, winner settled, seller paid. Every txid in `docs/evidence/testnet-jck1vba7/txids.md`, checkable on
   any testnet explorer; the result page shows the proof and the note-opening disclosure.
2. The disclosure verifier against a public lightwalletd:
   `bb-auction --network test --lightwalletd-url https://testnet.zec.rocks:443 --verify-disclosure disclosure.json`.
3. The audit tool catching a forged result: `docs/evidence/audit-tamper.md` (MISMATCH, exit 2).
4. The regtest money loop across formats and the FROST escrow: `docs/evidence/regtest/`.
5. `cargo test -p bb-frost -p bb-auction -p bb-verify-wasm` and the CI badge: everything green.

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
