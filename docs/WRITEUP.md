# Blind Bazaar + Go Dark — ZECATHON write-up

**One sentence.** Blind Bazaar runs sealed-bid tenders and auctions on Zcash where every bid is one shielded
payment with the amount inside its encrypted memo, so the losing bids are never revealed to anyone; Go Dark is the
pay button that lets a supplier or bidder on Solana or Ethereum take part without holding a transparent coin.

## Why Zcash, and only Zcash

A sealed-bid auction needs a bid nobody else can read, an auctioneer who can read it but not spend it, and a
provable outcome that reveals no loser. Zcash has all three as chain primitives: the encrypted memo carries the
bid, a full viewing key reads bids with no spend authority, and the note opening of the winning output checks
against its on-chain commitment. No contract is deployed: a lot is a fresh diversified unified address and a
viewing key. An Ethereum auction publishes every bid and bidder forever. This one publishes the winning price,
the winning payment and a proof, and nothing about the losers.

## What works today

- A seller lists a lot; the service derives an Orchard-only unified address for it. Deposits come from a short
  menu so a refund amount never marks its lot.
- A bidder pays a ZIP-321 payment link; the memo is `BB1|lot|bid|refund address|nonce`. The service recovers
  memos through lightwalletd, counts a deposit after two confirmations, re-checks counted bids against the chain
  on every pass, and lists `sha256(nonce)` receipts so a bidder sees "bid received" without revealing anything.
  A bid key held only by the bidder shows their standing on any device.
- The chain closes the lot: the first block after the close time decides what was late. Losers are refunded
  shielded after a random delay, in batched transactions that mix lots; the winner pays the balance by memo; the
  seller is paid.
- Four formats on the same rails: sealed first price, editions at a uniform price, Vickrey (the winner pays the
  second bid, proven from the ledger commitment with a Merkle path, so the winning bid stays sealed), and a
  private request for quotes.
- Sealed-reveal lots make bids commitments the operator cannot open until bidders reveal after close. Auditable
  lots publish the lot's own viewing key at close; `bb-auction audit` recomputes the result from the chain and
  says MISMATCH when the published one is wrong.
- Custody: the spender holds no spending key. Spend authority is a 2-of-3 FROST group key (dealer or DKG); every
  payout is a PCZT that independent signer hosts inspect, match against the viewer's ledger of expected payouts,
  cap per transaction and per day, and co-sign.
- The result page publishes a ZIP 311-style note-opening disclosure of the winning output, verifiable against
  any public lightwalletd. Optional email tells a bidder or seller when something needs them.
- Go Dark: a React component and a one-tag script. A recipient with no Zcash wallet can make one in the browser
  (standard 24 words, same key derivation as the mobile wallets, nothing stored or sent). Solana routes through NEAR Intents, which accepts a unified
  address at quote time; Ethereum routes through Maya Protocol, which since the Ironwood upgrade only pays a
  transparent receiver, so that hop is shielded on arrival and the widget says so. A "who sees what" panel precedes every confirm. For auctions, Go Dark swaps the
  deposit to the lot's address and relays the sealed bid as an envelope, because no swap rail can attach a memo
  to the ZEC it delivers.

## Privacy model and its limits, plainly

Other bidders see that a lot exists, its result and a list of receipt hashes. A chain observer sees
indistinguishable shielded transactions. The seller sees the winning amount. The rail sees the payer and the
Zcash destination, never the bid.

The known limit is operator trust: on a classic lot the operator reads bids while it is open and could shill. We
say so. Public receipts, the ledger hash committed at close, the winner's disclosure, the viewer/spender split
and the signer ledger check bound what the operator can do; sealed-reveal lots remove the early read; auditable
lots let anyone recompute the result. Refund timing is measured, not asserted (`docs/evidence/`). Where a rail
delivers to a transparent receiver we rotate addresses per payment, shield at once, and the widget says so.

## What is proven, and what is not

The full money loop ran on the public testnet twice (a classic lot and a sealed, auditable lot: independent
wallet bids, chain close, refund, settlement, payout, disclosure verified, audit from the published key) and on
a local regtest chain across the formats. The FROST escrow co-signed every payout on the regtest chain (two of three signers, the third
offline, ledger check on); it has not yet signed on a public chain. Distributed key generation and batched
refunds are unit-tested only. Only the bundled command-line wallet has placed a bid; no Go Dark
swap has yet moved real money. The README's honesty table keeps this list current, and the service parks itself
at the mainnet NU7 height so a mid-judging upgrade cannot leave a lot half settled.

## Roadmap

Chain runs of the FROST escrow and batched refunds; a mainnet micro-lot and wet Go Dark swaps; batch markets
(private order matching) on the same bid-by-memo rails.

## Tracks

Primary: private markets. Also: cross-chain (Go Dark), core and tooling (bid SDK, audit and disclosure verifiers).
