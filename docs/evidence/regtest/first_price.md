# regtest run: first_price (2026-10-22)

Local zeckit devnet (zebra + lightwalletd + faucet, 15 s blocks), `scripts/regtest-e2e.sh` with `defaults`. Every line below is the script's own output.

```
PASS  service synced; operator uregtest158r54tldc94fdgspsp6t73afz7k65nzzutduvc80067ggymye76qw9eryhelksqp4252dy004v6d9mjy0nyjj8gvdlp66azlz5dxm7tp; signing: seed
  faucet -> uregtest158r54tl… 20.0 ZEC (d7a40d40de9d)
PASS  faucet funded the operator
PASS  operator sees funds
PASS  operator sent 1 ZEC to each of 2 bidders (shielded sends work)
PASS  auction 8cq772th created (units=1, format first_price)
PASS  b1 paid the deposit with its sealed bid
PASS  b2 paid the deposit with its sealed bid
PASS  all 2 bids ingested from encrypted memos
PASS  winner is the 0.3 bid; ledger hash 0768640c0e539e9c…
PASS  note-opening disclosure verified against the chain
PASS  winning memo decrypted with the disclosed key; bid meets reserve
PASS  loser refund sent
PASS  b2 paid the balance (0.2 ZEC)
PASS  settlement detected
PASS  seller payout sent
PASS  CLI verifier: OK
```

Result: 16 passed, 0 failed.
