# Audit tool: does it catch a false result?

`bb-auction audit` recomputes an auditable lot's outcome from the chain alone, using the lot's published viewing
key, and compares it with what the operator published. A tool that only ever prints MATCH proves nothing, so this
records what it does when the published result is wrong.

## On the public testnet (Oct 18, 2026)

Lot `2xv644bv`, auditable, first-price, closed Oct 18 with no bids. Audit wallet scanned testnet from the lot's
birthday (4,515,321) to the tip (4,515,679) through `https://testnet.zec.rocks:443`.

Honest published result (what the service published):

```
  ledger root  MATCH  chain=e3b0c442…b855  published=e3b0c442…b855
  clearing     MATCH  chain=None  published=None
  winners      MATCH  chain=  published=
OK: the published result is what the chain says
exit 0
```

Tampered copy of the result (a price of 0.01234567 ZEC and an invented winning txid):

```
  ledger root  MATCH  chain=e3b0c442…b855  published=e3b0c442…b855
  clearing     MISMATCH  chain=None  published=Some(1234567)
  winners      MISMATCH  chain=  published=abab…abab
FAIL: the published result does not match the chain
exit 2
```

## Forgeries covered by unit tests (`services/auction/src/audit.rs`)

A simulated chain with three bids before close (3, 2 and 1 ZEC), a late bid, a short deposit, a reused nonce and a
memo-less payment. The judge applies the service's rules (`late`, `wrong_deposit`, `duplicate_nonce`) and the
comparison catches each of these published results:

| Forgery | What the tool reports |
|---|---|
| The winning 3 ZEC bid left out, runner-up declared winner at 2 ZEC, ledger root over the remaining bids | root, clearing and winner all MISMATCH |
| Right winner, price lowered to 2.5 ZEC | clearing MISMATCH, root and winner MATCH |
| Runner-up's txid published as the winner | winners MISMATCH |
| A bid the chain never carried added to the ledger (a planted shill) | ledger root MISMATCH |
| Sealed lot: a reveal whose amount does not match its commitment | that bid is `bad_reveal`, not counted |
| Sealed lot: no reveals published | every bid `unrevealed` |

## Oct 22: a lot with real bids

Sealed, auditable testnet lot `ejp8jz0s` (`docs/evidence/testnet-ejp8jz0s/`): two commitment bids from an
independent wallet, both revealed; the audit from the published key read both from the chain and matched root,
price and winner; a tampered result naming the runner-up at its own price was caught (MISMATCH, exit 2).

## What this does not cover
- An operator who withholds the viewing key: the tool cannot run. The key is published on the result page at
  close; a lot whose result page shows no key is not auditable, and the page says so.
- Bids the operator refuses to count because they were "late" when the chain says otherwise: the tool reads mined
  times from its own chain view, so a lie about timing shows up as a `valid` bid the operator left out (root
  MISMATCH), which is the first row above.

Reproduce: lot and result JSON from `GET /auctions/{id}` and `GET /auctions/{id}/result`, then
`bb-auction audit --ufvk <published key> --birthday <published birthday> --lot lot.json --result result.json`.
Edit any field of `result.json` and run again.
