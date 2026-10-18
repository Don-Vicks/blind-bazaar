# Wallet compatibility

Which wallets have actually placed a bid by paying a Blind Bazaar ZIP-321 URI with the memo intact. A row is
"tested" only when a bid from that wallet was read by the service on a chain and appears in a receipt list.

| Wallet | Network | Memo from a ZIP-321 QR | Tested | Notes |
|---|---|---|---|---|
| `bb-auction wallet` (bundled, `zcash_client_backend`) | regtest | yes | yes, Oct 13 (full money loop, `scripts/regtest-e2e.sh`) | A separate seed from the operator; this is the wallet the evidence runs use. |
| `bb-auction wallet` | testnet | yes | not yet: the operator wallet holds no testnet ZEC and the public faucets were down on Oct 17-18 | Command below. |
| Zodl (formerly Zashi), iOS/Android | mainnet | expected (ZIP-321 with memo) | no | No public testnet build. Needs the mainnet lot. |
| Zingo | mainnet, testnet | expected | no | Testnet build must carry the NU7 branch id (activated Oct 6) or it cannot send. |
| YWallet | mainnet, testnet | expected | no | Same NU7 caveat on testnet. |

Until a row above says "tested", the site's "scan with Zodl, Zingo or YWallet" is a claim, not evidence, and the
lot page says so. The bundled wallet is the proven path:

```bash
# a fresh wallet with its own seed; prints its address; waits for funds; pays the bid URI with the memo
bb-auction wallet --dir ./bidder --seed-file ./bidder.seed --wait-balance 10000000 --pay "zcash:utest1…?amount=0.1&memo=…"
```

Get the URI from the lot page (Copy URI) or from `GET /auctions/{id}/bid-uri?bid_zats=…&refund_address=…`.
