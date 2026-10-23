# Public testnet tender 08t6ynq2 (Oct 23, 2026): sealed quotes, lowest wins, nobody learns the other price

A buyer posted a job ("40 hours of Rust security review") with a budget of 0.05 TAZ and a 0.01 TAZ bond. Two
suppliers quoted from an independent wallet (the bundled `bb-auction wallet`, its own seed): 0.03 and 0.025 TAZ,
each quote sealed inside the memo of a shielded bond payment. The chain closed the round, the lower quote won, the
buyer paid the winning quote by memo from a third wallet, the winning supplier received quote plus bond, the losing
supplier's bond came back, and the losing quote was never revealed to anyone. Every file here is the API's own
JSON or the verifier's output.

| # | UTC | step | txid | on chain | what it proves |
|---|---|---|---|---|---|
| 1 | 13:55 | quote 0.03 TAZ sealed in the memo, 0.01 bond | 532390f3b988b1e1d11fa7323a118a0cc025654936a5927d95cdf386db4a05f7 | shielded, no amount or memo visible | a quote is one ordinary shielded payment |
| 2 | 13:57 | quote 0.025 TAZ sealed, 0.01 bond | 9fdd59ef7e32aedca48a0d987046bfa5938c9428f254a086a9bee6d96c0b7c30 | shielded | the second supplier |
| 3 | 14:11 | close at block 4,532,675; lowest quote wins at 0.025 | `ledger_hash` eaa3f922da80427be09a68ad5c352c33d748a691942172844976961731d14d4b | | the chain decided the close; the root commits both quotes |
| 4 | 14:13 | the buyer pays the winning quote (`BB1S` memo, from the buyer's own wallet) | 3632a5ccb1fcf77a89f3cdff6302bb963170c2ae6302827fe448e1d8fc4f4817 | shielded | settlement matched to the winning nonce |
| 5 | 14:1x | winning supplier paid 0.035 (quote plus bond) | 64a30a9ecbb12e08a81f96b50909edfeb7471a42c51ac9040d55d47d7ce93a39 | shielded | payout to the address sealed in the winning memo |
| 6 | 14:1x | losing supplier's bond returned | 58186ba83646c2462027c1bf93b3b85cd666ea9a29358d660c77a799338e7e2f | shielded | the losing quote stays sealed; only the bond moves |

Disclosure of the winning payment verified against the chain by the CLI (`verify-cli.txt`). The result page shows
`reserve_rule: at_most`: the winning quote is proven to be within the budget without revealing the other quote.

## Not shown

- A supplier quoting from a mobile wallet; both quotes came from the bundled command-line wallet.
- A sealed-reveal tender (commitments instead of amounts): that mode ran on the auction lot `ejp8jz0s`; the
  same code applies to `rfq` lots with `reveal_secs` set, not yet exercised on a chain.
