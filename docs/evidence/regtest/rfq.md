# regtest run: rfq (FORMAT=rfq, lowest sealed quote wins, requester settles) (2026-10-22)

Local zeckit devnet (zebra + lightwalletd + faucet, 15 s blocks), `scripts/regtest-e2e.sh` with `defaults`. Every line below is the script's own output.

```
PASS  service synced; operator uregtest1m6pspwem0vt5mfkr7p446va00pezvazr5u5w0mcw2ac5rqpjvsu50xmcdmnqfxx6kzdlxmfmk8xe9qzl7te9p6qza3myjgph6vsvxs2z; signing: seed
  faucet -> uregtest1m6pspwe… 20.0 ZEC (ff1299fa8bf4)
PASS  faucet funded the operator
PASS  operator sees funds
PASS  operator sent 1 ZEC to each of 3 bidders (shielded sends work)
PASS  auction 7anyd90t created (units=1, format rfq)
PASS  b1 paid the deposit with its sealed bid
PASS  b2 paid the deposit with its sealed bid
PASS  all 2 bids ingested from encrypted memos
PASS  rfq: the 0.2 quote beats 0.25 under a 0.3 budget; ledger hash 96c5f9d008d52496…
PASS  note-opening disclosure verified against the chain
PASS  winning memo decrypted with the disclosed key; quote is within budget
PASS  loser refund sent
PASS  requester paid the 0.2 winning quote
PASS  settlement detected
PASS  winning quoter paid 0.2 quote + 0.1 bond
PASS  CLI verifier: OK
```

Result: 16 passed, 0 failed.
