# regtest run: vickrey (FORMAT=vickrey, winner pays the second bid) (2026-10-22)

Local zeckit devnet (zebra + lightwalletd + faucet, 15 s blocks), `scripts/regtest-e2e.sh` with `defaults`. Every line below is the script's own output.

```
PASS  service synced; operator uregtest1cnh4mc3tlctqvjxetw5zyy4yrsjztc3gglapnx2xefytzslkhfmhnuk93czmpeanqnwntsfx395mscrck4tklss35w0vdf65fghn7vw5; signing: seed
  faucet -> uregtest1cnh4mc3… 20.0 ZEC (fa1fbdc9858e)
PASS  faucet funded the operator
PASS  operator sees funds
PASS  operator sent 1 ZEC to each of 2 bidders (shielded sends work)
PASS  auction e4sb4aqy created (units=1, format vickrey)
PASS  b1 paid the deposit with its sealed bid
PASS  b2 paid the deposit with its sealed bid
PASS  all 2 bids ingested from encrypted memos
PASS  vickrey: 0.3 bid wins and pays the 0.25 runner-up; price proof roots to the ledger; winning bid stays sealed
PASS  note-opening disclosure verified against the chain
PASS  vickrey: winning memo stays sealed (no key disclosed)
PASS  loser refund sent
PASS  b2 paid the balance (0.15 ZEC)
PASS  settlement detected
PASS  seller payout sent
PASS  CLI verifier: OK
```

Result: 16 passed, 0 failed.
