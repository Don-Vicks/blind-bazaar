# regtest run: frost (FROST=1, 2-of-3 escrow, signer 3 offline) (2026-10-22)

Local zeckit devnet (zebra + lightwalletd + faucet, 15 s blocks), `scripts/regtest-e2e.sh` with `defaults`. Every line below is the script's own output.

```
PASS  frost keygen done; signers 1 and 2 up, signer 3 deliberately absent
PASS  service synced; operator uregtest1hg4j6pfmf8h3rj3efnltlfwme5e7ns0ap4kj66sehke8qufqt8qehs869d4648y366jewy0jtxqvhe7fvqszg00n2zdpfh2zlgqnur5m; signing: frost 2-of-3 (3 signers configured)
  faucet -> uregtest1hg4j6pf… 20.0 ZEC (6b01fb9f3c21)
PASS  faucet funded the operator
PASS  operator sees funds
PASS  operator sent 1 ZEC to each of 2 bidders (shielded sends work)
PASS  every send was co-signed by signers 1 and 2 (2-of-3); the spender holds no spending key
PASS  auction 7sr118ep created (units=1, format first_price)
PASS  b1 paid the deposit with its sealed bid
PASS  b2 paid the deposit with its sealed bid
PASS  all 2 bids ingested from encrypted memos
PASS  winner is the 0.3 bid; ledger hash 6988ef3d59a3b197…
PASS  note-opening disclosure verified against the chain
PASS  winning memo decrypted with the disclosed key; bid meets reserve
PASS  loser refund sent
PASS  b2 paid the balance (0.2 ZEC)
PASS  settlement detected
PASS  seller payout sent
PASS  CLI verifier: OK
```

Result: 18 passed, 0 failed.
