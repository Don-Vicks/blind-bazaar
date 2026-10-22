# Public testnet lot ejp8jz0s (Oct 22, 2026): sealed-reveal, auditable, audited from the chain

A sealed-reveal lot with its own wallet account. Two commitment bids from an independent wallet (the bundled
`bb-auction wallet`, its own seed), each memo carrying only `sha256(lot|bid|salt)`; the chain closed the lot;
the operator could not read either amount until the bidders revealed during the window; the lot's viewing key
was published at close; `bb-auction audit` rescanned the public testnet from the published birthday with that key,
read both commitments, applied the published reveals and reproduced the root, the price and the winner. A
tampered copy of the result was caught. Reserve 0.02 TAZ, deposit 0.01 TAZ, reveal window 10 minutes.

| # | UTC | step | txid or value | what it proves |
|---|---|---|---|---|
| 1 | 20:05 | bid A: commitment to 0.03 TAZ, 0.01 deposit | 59ec5bba2a68480f4456a728059db45623c7e9285c9b6e81301c9cc2863e431b (block 4,530,073) | the memo carries a hash, not an amount |
| 2 | 20:07 | bid B: commitment to 0.025 TAZ, 0.01 deposit | 3bf92156f95bedca7b85d6f99a3301c610fdc16ad9890a16a9c2bb58e9072e2b (block 4,530,076) | second sealed bid |
| 3 | 20:20 | close by the chain at block 4,530,107; commitments fixed; reveal window opens | `commit_hash` 88b2377b8e7adddb575b8225e9ecdb86ef9f009fc37c52753395b07a32317549 | the set of bids is fixed before any amount is known |
| 4 | 20:21 | both bidders reveal (`POST /auctions/{id}/reveals` with bid key, amount, salt) | receipts d8999b38… and 6e81a7be… | reveals must match the committed hash |
| 5 | 20:31 | window ends; result: A wins at 0.03 | `ledger_hash` cc67e1ddb780137c783c99687d1f295deb7a42a84b25d37a9fcde2043d64d17c | ranking over revealed amounts only |
| 6 | 20:31 | lot account swept to the operator (auditable lots keep deposits in their own account) | 9c5ffe254bddabb99d812a0fe9dd0697698b02856ff1ed1ce48e8693ada4cd8b | the only spend an auditor sees from the lot's key |
| 7 | 20:32 | winner pays the 0.02 balance by memo | 17cbb1e036c5ae5cc2178e3c9f24102489844fbce0ae42fe09bee1535961cea6 | settlement matched to the winning nonce |
| 8 | 20:36 | seller paid 0.03 | 73b78f72b17b0ead0ac3ee245f471bb08ff594677a6e37725cc4f9d13722ebd5 | payout to the seller's own wallet |
| 9 | 20:37 | loser refunded 0.01 after the random delay | 28f196cd6bcc938ef2872b7d80005b95cfda7828f5918ab9164250052413b07f | the refund goes to the address sealed in the losing commitment memo |

## The audit, from the chain alone

Published at close on the result page: the lot's unified full viewing key, birthday 4,530,063, and both reveals.
`audit-honest.txt` (scan through `https://testnet.zec.rocks:443`, nothing from the service trusted beyond the
public lot definition):

```
  59ec5bba2a68480f valid h=Some(4530073) value=1000000 bid=3000000 d8999b381b7e
  3bf92156f95bedca valid h=Some(4530076) value=1000000 bid=2500000 6e81a7be83ab
2 received outputs, 2 bids read, 0 rejected, 2 valid
  ledger root  MATCH
  clearing     MATCH  chain=Some(3000000)  published=Some(3000000)
  winners      MATCH  chain=59ec5bba…  published=59ec5bba…
OK: the published result is what the chain says        exit 0
```

`result-tampered.json` declares the runner-up the winner at its own 0.025 price. `audit-tampered.txt`:

```
  ledger root  MATCH
  clearing     MISMATCH  chain=Some(3000000)  published=Some(2500000)
  winners      MISMATCH  chain=59ec5bba…  published=3bf92156…
FAIL: the published result does not match the chain    exit 2
```

Reproduce: the `command` field of `result.json` is the exact invocation; swap `--result` for the tampered file.

## Not shown

- A consumer wallet placing the commitment bid (the bundled wallet built the `BB1C` memo; the browser does the same
  through the SDK).
- An unrevealed bid: both bidders revealed. The rule (unrevealed bids are refunded and cannot win) is unit-tested.
