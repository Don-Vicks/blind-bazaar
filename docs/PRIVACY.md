# Privacy model and rail findings

## Who sees what

| Party | Sees | Does not see |
|---|---|---|
| Other bidders | That an auction exists, its result, and the list of bid receipts (hashes) | Any bid, any bidder, which receipt is whose |
| Public chain observer | Ironwood-pool shielded transactions of unknown amount and origin | Which transactions belong to the auction |
| Auction operator | Decrypted memos via the auction viewing key; bid envelopes from Go Dark | Spend authority over bidder funds beyond the deposit |
| Seller | Winning amount and payout | Losing bids and losing bidders |
| Source chain (Go Dark) | A swap from the bidder's wallet to the rail | The Zcash destination memo or bid |
| Swap rail | Bidder's source address, the ZEC destination address | The bid (it travels in the envelope, not through the rail) |

## Operator trust and the viewer/spender split

Whoever holds the auction viewing key can read bids while the auction is open. Mitigations shipped:
receipts (sha256 of the bidder's nonce) published as bids arrive, a hash of the full bid ledger
published at close, and a note-opening disclosure of the winning transaction.

The service runs in one of three roles (`ROLE=all|viewer|spender`):

- **viewer** holds only the unified full viewing key. It ingests bids, runs the auction clock,
  publishes receipts, results and the disclosure, and serves the public API. It cannot move funds.
- **spender** holds the spending key and its own wallet. It mirrors each auction definition from
  the viewer exactly once (later changes are ignored), ingests the same bids from its own chain
  view, recomputes winners and payouts with the same deterministic rules, signs the payouts, and
  reports the transaction ids back. It never executes a payout the viewer tells it to; only payouts
  its own ledger derives.
- **all** runs both in one process (development and the hackathon demo).

With the split on two hosts, a compromised viewer can misreport or censor but cannot steal: every
refund goes to the address inside the bidder's own encrypted memo, every seller payout is bounded
by the winning bid the spender decrypted itself, and the seller address is pinned at first sight.
A compromised spender alone can no longer steal when it runs under the FROST escrow: the spend
authority is a 2-of-3 group key, the spender holds only the viewing key, and two signer hosts
inspect every transaction's destinations and amounts before co-signing (`docs/FROST.md`). Theft
needs the spender plus a signer, or two signers.

## Sealed-reveal: hiding bids from the operator too

The stated limit above, that whoever holds the viewing key reads bids while a lot is open, is removed by
sealed-reveal lots. The memo carries a commitment to the amount; the viewing key learns that a deposit arrived
with a receipt, and nothing else. At close the commitment root is published first, so a bidder cannot change
their mind after seeing anything, and the operator cannot drop a commitment unnoticed. Then each bidder reveals
`(bid, salt)` with their bid key, the service checks it against the commitment, and the lot closes over the
revealed bids with the same ledger root, pricing and proofs as a plain lot. A bidder who does not reveal gets
the deposit back in full and cannot win, so there is no penalty beyond losing the chance to win. The cost is a
second step for bidders and a delay equal to the reveal window; the classic mode remains for sellers who prefer a
result the moment the lot closes.

## Cross-chain bids use an envelope, not the memo

No swap rail can attach a memo to the ZEC it delivers. Go Dark therefore sends the
fixed deposit to the auction address through the rail and posts the same `BB1` payload
to the auction service as an envelope keyed by the rail's deposit address. The service
matches the rail's destination txid to the shielded note it received. The operator is
the only reader in both paths, so the trust model is unchanged.

## Day-one rail spikes (Oct 13, 2026)

- Maya `quote/swap` ETH.ETH -> ZEC.ZEC with `destination=u1...`: accepted. Memo returned
  as `=:z:u1...`, total 24 bps, outbound fee 39,198 zat, 24 s. Inbound address
  `0xaaa8600f4a69c745a8d8a77c882b8962914d0d90`, router `0xe3985E6b61b814F7Cdb188766562ba71b446B46d`.
  Not yet verified: that the outbound actually lands as a shielded output (needs a wet test).
- NEAR Intents 1Click `POST /v0/quote` `dry:true`, `EXACT_OUTPUT` 0.1 ZEC, Solana USDC in,
  `recipient=u1...`: accepted (amountIn ~123.3 USDC, withdrawFee 32,000 zat, 135 s).
  Same with `t1...`: accepted. Official docs still say transparent-only; a wet test
  with a small amount decides whether `u1` delivery is shielded.
- Price reality: 0.1 ZEC is about 121 USD, so demo deposits are 0.01 ZEC or less.

## Network upgrade timing

Testnet is on NU7 (since height 4,465,026). Mainnet is on NU6.3 (Ironwood) with NU7 expected around Nov 5, 2026.
The service depends on the pre-release librustzcash line that understands both, and the mainnet demo must be
recorded before NU7 activates there or re-tested after.

## Close is decided by the chain

A bid is late if the block that mined it has a timestamp after the lot's `close_at`; the closer waits until a
later block has been scanned before fixing the ledger, and records that `close_height`. Ties between equal bids
go to the bid mined first (height, then txid, then output index). None of this depends on when a process saw a
transaction, so the viewer and the spender, reading the same chain, always close on the same ledger and the same
winner.

## Ledger commitment and inclusion proofs

At close the ledger hash is the root of a Merkle tree over every valid bid: leaves are
`sha256(0x00 || receipt || bid_zats_be8 || nonce)` sorted by receipt, nodes are `sha256(0x01 || left || right)`, an
odd node pairs with itself. The nonce blinds the leaf: receipts are public and bid amounts are small, so without it a
leaf could be brute-forced to an amount in seconds. After close, `GET /auctions/{id}/proof?receipt=…` returns the
sibling path for one receipt. The bidder's browser recomputes the root from their receipt, their own bid amount and
nonce (never sent) and the path, and compares it with the published hash. A bidder therefore learns only that their
own bid was counted at the amount they sealed; nobody learns anyone else's bid, and the service cannot drop or alter a
bid after close without changing the root. A Vickrey price proof publishes the runner-up's receipt, amount and nonce,
which reveals that one amount and nothing else about them.

## Reserve-met proof (optional, `DISCLOSE_WINNING_MEMO=true`)

The note-opening disclosure proves the amount and recipient of a winning output, but the bid itself lives in the
memo, which the note commitment does not cover. With this option the service also discloses the symmetric key of each
winning note, derived from the viewing key for that one action. Anyone can then decrypt that note's ciphertext from
the transaction, read the memo, and check the bid against the reserve. The key opens nothing else.

Cost: the winner's memo becomes public, which reveals the winning bid (already public as the price for single lots,
above the clearing price for editions) and the winner's refund address. Bidders who care should use a fresh
diversified address per bid; losing memos are never opened.
