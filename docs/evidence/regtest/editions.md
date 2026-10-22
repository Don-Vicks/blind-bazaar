# regtest run: editions (UNITS=2, uniform price, three bidders) (2026-10-22)

Local zeckit devnet (zebra + lightwalletd + faucet, 15 s blocks), `scripts/regtest-e2e.sh` with `UNITS=2`. Every line below is the script's own output.

```
PASS  service synced; operator uregtest1mmpmlkd039pkgzzpukx6xdqg6cd83wkhzjlc8zne8p8jnql5gn0gqk8j356gckmzq5au0djspq9778t7puq36zxdpk6pl5wysvc6k6q4; signing: seed
  faucet -> uregtest1mmpmlkd… 20.0 ZEC (67ee72e15c0e)
PASS  faucet funded the operator
PASS  operator sees funds
PASS  operator sent 1 ZEC to each of 3 bidders (shielded sends work)
PASS  auction 4x6pz7xp created (units=2, format uniform)
PASS  b1 paid the deposit with its sealed bid
PASS  b2 paid the deposit with its sealed bid
PASS  b3 paid the deposit with its sealed bid
PASS  all 3 bids ingested from encrypted memos
PASS  edition: top two win (0.3 and 0.25) at clearing 0.25; 0.22 loses; ledger 31cbd8321c42eb15…
PASS  note-opening disclosure verified against the chain
PASS  winning memo decrypted with the disclosed key; bid meets reserve
PASS  loser refund sent
PASS  b2 paid the balance (0.15 ZEC)
PASS  b1 paid the balance (0.15 ZEC)
PASS  settlement detected
PASS  seller payout sent
PASS  CLI verifier: OK
```

Result: 18 passed, 0 failed.
