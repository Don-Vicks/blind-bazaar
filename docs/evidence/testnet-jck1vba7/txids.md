# Public testnet lot jck1vba7 (Oct 22, 2026)

Two bids from an independent wallet (the bundled `bb-auction wallet`, its own seed, funded by the fauzec faucet),
read by the service through `https://testnet.zec.rocks:443`. Operator float: 1 TAZ from the same faucet.

Lot: first price, reserve 0.02 TAZ, deposit 0.01 TAZ, seller address from a third wallet (`data/seller`).

| # | UTC | step | txid | pool, amount visible? | what it proves |
|---|---|---|---|---|---|
| 0 | 19:08 | faucet funds the operator | dbfe69818c38f64f047f02928e9cbd7b0a19b857c198c671982d4fc35c037b9d | shielded | float for refunds and fees |
| 0 | 19:11 | faucet funds the bidder | b6248884118bc2f104145e687055c26090e47579bb05824092add278ac2e7882 | shielded | the bidder is a separate wallet |
| 1 | 19:14 | bid A, 0.03 TAZ sealed in the memo, 0.01 deposit paid | e3a1be8fcab994d6ac90c5e239d78cac19c7c8d532e03b6bd1e8bb4005ee1557 | shielded, no amount | a bid is one ordinary shielded payment |
| 2 | 19:15 | bid B, 0.025 TAZ sealed, 0.01 deposit paid | 60af4ab5863d5b5d7e49402fd9fe6f957b1afad33d8a3e0bb3c895dd07b13ba6 | shielded, no amount | second bid from the same wallet, different key |
