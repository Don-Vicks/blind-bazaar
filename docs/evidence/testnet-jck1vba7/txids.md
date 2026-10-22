# Public testnet lot jck1vba7 (Oct 22, 2026): the full money loop on a public chain

Two bids from an independent wallet (the bundled `bb-auction wallet`, its own seed, funded by the fauzec
faucet), read by the service through `https://testnet.zec.rocks:443`; closed by the chain; loser refunded;
winner settled; seller paid; the winning note's opening published and verified against the chain by the CLI.
Operator float: 1 TAZ from the same faucet. Lot: first price, reserve 0.02 TAZ, deposit 0.01 TAZ, seller address
from a third wallet. Every file here is the API's own JSON (`lot.json`, `result.json`, `receipts.json`,
`events.json`, `disclosure.json`) or the verifier's output (`verify-cli.txt`).

| # | UTC | step | txid | on chain | what it proves |
|---|---|---|---|---|---|
| 0 | 19:08 | faucet funds the operator | dbfe69818c38f64f047f02928e9cbd7b0a19b857c198c671982d4fc35c037b9d | shielded | float for refunds and fees |
| 0 | 19:11 | faucet funds the bidder | b6248884118bc2f104145e687055c26090e47579bb05824092add278ac2e7882 | shielded | the bidder is a separate wallet |
| 1 | 19:14 | bid A: 0.03 TAZ sealed in the memo, 0.01 deposit | e3a1be8fcab994d6ac90c5e239d78cac19c7c8d532e03b6bd1e8bb4005ee1557 | shielded, no amount, no memo visible | a bid is one ordinary shielded payment |
| 2 | 19:15 | bid B: 0.025 TAZ sealed, 0.01 deposit | 60af4ab5863d5b5d7e49402fd9fe6f957b1afad33d8a3e0bb3c895dd07b13ba6 | shielded | second bid, different bid key |
| 3 | 19:34 | close at block 4,530,003 (first block after `close_at`) | no transaction; `ledger_hash` 0c7a31c9f5e36cd45fe992f800e62dba3967d5ddc8278fd9f99c56efd201123c | | the chain decided the close; the ledger root commits both bids |
| 4 | 19:37 | winner pays the 0.02 balance (`BB1S` memo) | f258e33378d2396dcedaf9d81fa23636ca1a0fa5de0bd56067505eaf153030b1 | shielded | settlement by memo, matched to the winning bid's nonce |
| 5 | 19:45 | seller paid 0.03 | 9c4e120d6a122d1add32454293c56bb991f5df467f4312263cc5bce7a27950b1 | shielded | payout to the seller's own wallet |
| 6 | 19:47 | loser refunded 0.01 after the random delay | 0e2f2de7b397568c3e918ed9e503ae3b1a7e15dec2f4408897e652bb6faa99d6 | shielded | the refund goes to the address sealed in the losing memo |

Disclosure: the winning output is action 0 of bid A's transaction, in the Ironwood pool (post-NU6.3), value
1,000,000 zatoshis, `cmx 1ccba1b378b7d2eef220368dddd15294ede833a98fc20ffc4ee65e4809ae8f2a`. `verify-cli.txt`:

```
ok   action 0 of e3a1be8f… (ironwood pool): 1000000 zatoshis to utest1zz93g27qu9awwp
OK: all 1 disclosure(s) match the chain
```

Reproduce the check: `bb-auction --network test --lightwalletd-url https://testnet.zec.rocks:443 --verify-disclosure disclosure.json`.

## What this run found and fixed

- The disclosure builder read only the `orchard_received_notes` table; since NU6.3 a note received through an
  Orchard receiver is stored under Ironwood. The first close logged "no received note" and published no
  disclosure. Fixed the same hour (commit c49dfd9): both tables are read, and a lot that closed without a
  disclosure gets it rebuilt on a later pass, which is how this lot's disclosure appeared at 19:47.
- The bundled wallet's bid came from the server-side `bid-uri` endpoint, which makes the nonce itself; such a bid
  has no bid key, so its standing page cannot be opened. Browser bids carry a key. Settlement still works (it is
  by memo), as this run shows.

## Not shown

- A consumer wallet (Zodl, Zingo, YWallet) placing the bid: the bidder here is the bundled command-line wallet.
- The seller's and bidder's wallets receiving the payout and refund as seen from their side (their txids are on
  chain; the wallets were not opened to display them).
