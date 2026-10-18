# Evidence

Everything a judge can check without trusting us. Each run gets a folder named by network and lot id. Nothing is
typed by hand: result and disclosure files are the API's own JSON, txids are read back from the service.

| Folder | Network | What it shows | Status |
|---|---|---|---|
| `regtest/` | local zeckit regtest | full money loop: funding, two or three bids, close, refunds, settlement, seller payout, CLI verifier | first-price run passed Oct 13; `UNITS=2`, `FORMAT=vickrey`, `FORMAT=rfq`, `FROST=1` written, not yet run |
| `testnet-<lot>/` | Zcash testnet (NU7) | public lot with real wallet bids | not yet run |
| `mainnet-<lot>/` | Zcash mainnet | public micro-lot, published txids | not yet run |
| `go-dark/` | Solana or Ethereum → Zcash mainnet | one completed swap per rail with the delivery txid and delivery mode | not yet run |

## What each run folder contains

- `txids.md`: the table below.
- `result.json`, `receipts.json`, `events.json`: `GET /auctions/{id}/result`, `/receipts`, `/events` as returned.
- `disclosure.json`: the `disclosure` field of the result, as fed to the verifier.
- `proof-<receipt>.json`: `GET /auctions/{id}/proof?receipt=` for a team-owned bid, with the nonce so anyone can
  recompute the blinded leaf.
- `verify-cli.txt`: output of `bb-auction --lightwalletd-url <public node> --verify-disclosure disclosure.json`.
- `screenshots/`.

## Txid table format

| # | UTC time | height | step | chain | txid or signature | pool, amount visible? | what it proves | how to verify |
|---|---|---|---|---|---|---|---|---|

Rows, in order: each bid; a Go Dark source transaction and its Zcash delivery (and auto-shield, if the rail
delivered transparently); close (no transaction: `close_height` and the ledger root); each refund; the seller
payout. Publishing refund and payout txids deliberately links them to the lot; only team-owned wallets are used
for published runs.

## Verifier commands

```bash
curl -s https://HOST/bb/auctions/ID/result | jq .disclosure > disclosure.json
bb-auction --lightwalletd-url https://zec.rocks:443 --verify-disclosure disclosure.json
curl -s https://HOST/bb/auctions/ID/result | jq '.price_proof.proof.root == .ledger_hash'
```

The browser equivalents are `/a/ID/verify` and the status page's inclusion check.
