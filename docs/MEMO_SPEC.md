# BB1 memo format

Every bid is a shielded transaction to the auction's unified address (Orchard receiver,
Ironwood pool). The 512-byte encrypted memo carries the bid as ASCII:

```
BB1|<auction_id>|<bid_zatoshis>|<refund_address>|<nonce>
```

| Field | Format | Example |
|---|---|---|
| `BB1` | literal version tag | `BB1` |
| `auction_id` | 8 lowercase base32 chars (crockford) | `7k2m9q4e` |
| `bid_zatoshis` | decimal integer, no separators | `150000000` (1.5 ZEC) |
| `refund_address` | unified address (`u1...`/`utest1...`) with an Orchard receiver | `u1...` |
| `nonce` | 16 lowercase hex chars: the first 16 hex of `sha256(secret)`, where `secret` is 16 random bytes (32 hex) the bidder's browser keeps and never sends in the memo | `9f3a0c7b12de44aa` |

Rules:
- The transaction value must equal the auction's fixed deposit, or the bid is
  `malformed` and the deposit is refunded to `refund_address` if parseable.
- A memo that does not start with `BB1|` or has the wrong field count is ignored for
  ranking and refunded if a refund address can be recovered.
- The secret authenticates the bidder to the service (`/standing`, `/fulfilment`). The nonce may become
  public if the winning memo is disclosed; the secret never does.
- Receipt = lowercase hex `sha256(nonce)`. The service publishes receipts of received
  bids; a bidder checks theirs without revealing anything.
- Winner settlement memo: `BB1S|<auction_id>|<nonce>` with value = `bid - deposit`.
- Sealed-reveal bid memo: `BB1C|<auction_id>|<commitment>|<refund_address>|<nonce>` where `commitment` is the
  lowercase hex `sha256("<auction_id>|<bid_zatoshis>|<salt>")` and `salt` is 16 random bytes (32 hex) kept by the
  bidder. Used only on lots created with `reveal_secs > 0`; a plain `BB1` on such a lot is refunded, and a `BB1C`
  on a plain lot is refunded. The reveal is `POST /auctions/{id}/reveals {secret, bid_zats, salt}` during the
  reveal window; it is not a chain transaction. Pinned vector: `sha256("ab|1|00") =
  44b0ebb0eba21fe71bc5f2b2df7753b86271bf1edc7ee876d4406f1f4efff064`.
- Keep memos under 300 bytes. Encode in ZIP-321 as base64url without padding.
