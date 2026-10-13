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

## Known limitation: operator trust

The operator can read bids while the auction is open. Mitigations shipped: receipts
(sha256 of the bidder's nonce) published as bids arrive, a hash of the full bid ledger
published at close, and a note-opening disclosure of the winning transaction. v2: split
the viewing key and the spending key across two operators.

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
