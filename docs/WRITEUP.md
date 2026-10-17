# Blind Bazaar + Go Dark — ZECATHON write-up (draft, under 800 words)

**One sentence.** Blind Bazaar is a sealed-bid auction house where every bid is a shielded Zcash transaction, and
Go Dark is the one-click on-ramp that lets anyone on Solana or Ethereum enter it without ever holding a
transparent coin.

## Why Zcash, and only Zcash

A sealed-bid auction needs three things: a way to submit a bid nobody else can read, a way for the auctioneer to
read it without being able to spend it, and a way to prove the outcome without revealing the losers. Zcash has all
three as chain primitives. The 512-byte encrypted memo carries the bid. A full viewing key lets the auction service
decrypt incoming bids while holding no spend authority over them. The note opening of the winning output can be
published and checked against the on-chain commitment, so the result is provable while every losing bid stays
sealed forever. No smart contract is deployed: the auction is a fresh diversified unified address and a viewing
key. Ethereum auctions leak every bid, bidder and losing amount to the world. This one reveals the winning price
and the winning payment, and nothing about the losers; the one party that reads bids before close is the
operator, and the table below says exactly what it can and cannot do with that.

## What works today

- Seller lists a lot; the service derives an Orchard-only unified address for it.
- Bidders scan a ZIP-321 QR from Zodl, Zingo or YWallet. The memo is `BB1|auction|bid|refund address|nonce`.
- The service syncs with lightwalletd, enhances transactions to recover memos, validates deposits and nonces,
  and publishes `sha256(nonce)` receipts so a bidder can confirm "bid received" without revealing anything.
- At close: highest valid bid at or above reserve wins; a hash of the bid ledger is published; losers are
  refunded shielded with random delays; the winner pays the balance by memo; the seller is paid.
- Four formats on the same rails: sealed first price, editions at a uniform price, Vickrey (the winner pays
  the second bid, the winning bid stays sealed, and the price is proven from the ledger commitment with a Merkle
  path), and a private request for quotes where suppliers seal quotes against a published budget and only the
  winning price is revealed.
- Custody: the spender holds no spending key. The escrow's Orchard spend authority is a 2-of-3 FROST group key;
  every payout is a PCZT that independent signer hosts inspect and co-sign.
- The result page publishes a ZIP 311-style note-opening disclosure of the winning output. Anyone can verify it
  with `bb-auction --verify-disclosure` against a public lightwalletd. (Mainnet auction link: TODO.)
- Go Dark: `<GoDark/>` React component and a `<script>` bundle. Ethereum routes through Maya Protocol
  straight to a unified address; Solana routes through NEAR Intents. A "who sees what" panel precedes every
  confirm. For auctions, Go Dark swaps the deposit to the auction address and relays the sealed bid as an
  envelope, because no swap rail can attach a memo to the ZEC it delivers.

## Privacy model and its limits, plainly

Other bidders see that an auction exists, its result, and a list of receipt hashes. A chain observer sees
indistinguishable shielded transactions. The seller sees the winning amount and the payout. The source chain sees
a swap to a rail; the rail sees the payer and the Zcash destination, never the bid.

The known limit is operator trust: whoever runs the service can read bids while the auction is open and could
shill. We do not hide this. We mitigate it with public receipts as bids arrive, a ledger hash committed at close,
and the disclosure of the winner. The viewer/spender split is shipped: the viewing key and the spending authority sit in two processes so neither
can both read and act. Cross-chain bids are trusted identically to memo bids because the operator is the only
reader in either path. Delivery to a unified address through NEAR Intents is accepted at quote time but documented
as transparent-only; where a rail delivers to a transparent receiver we rotate addresses per payment and shield
immediately.

## Go Dark as open infrastructure

Any creator, merchant or DAO can drop one tag on a page and start receiving shielded ZEC from users who have
never touched Zcash. The widget never sees a seed. (In-browser wallet generation for receivers without a Zcash wallet is not built; today a recipient pastes a unified address from Zodl, Zingo or YWallet.) Planned text, kept for honesty: receivers would get a 24-word phrase generated in
the browser and an import path into Zodl or Zingo. Every site that adds the button onboards its own users into
the shielded pool.

## Primitives used

Ironwood-pool shielded transactions to Orchard receivers; encrypted memos; unified full viewing keys; fresh
diversified unified addresses per auction and per payment; ZIP-321 payment URIs; note-opening disclosure.

## Roadmap

Distributed key generation for the escrow, signer-side payout verification against the viewer's ledger, Dutch
format, and batch markets (private order matching) on the same bid-by-memo rails.

## Tracks

Primary: Private markets. Also: Cross-chain (Go Dark), Core & tooling (bid-commit SDK, viewing-key poller,
disclosure verifier).
