# Security review of the week's additions (Oct 19, 2026)

A read-only review of everything added since Oct 12 (freeze, deposit menu, notifications, reveals, envelopes,
standing, ledger endpoint, rolling cap, Go Dark proxy routes, bid cards) was run by a separate agent against
commit 6fea5d4, then each finding was verified in the code and fixed the same day. Nothing below was found on a
chain; these are code-level defects.

| # | Severity | Finding | Fix |
|---|---|---|---|
| 1 | high | A signer's ledger check consumed an expected payout only at round 2, so two sessions run side by side could both match the same refund and get it signed twice. | Round 1 now also excludes payouts matched by any unexpired session (`SignerHost::reserved`). Test: a second live session for the same payout is refused. |
| 2 | medium | The Go Dark proxy keyed its rate limit on `x-forwarded-for`, which any client can set, and cleared every bucket when the table filled. | The header is honoured only with `GODARK_TRUST_PROXY=true` (set in the Caddy deployment); otherwise `x-real-ip` or one shared bucket. The table evicts its oldest entries instead of clearing. |
| 3 | medium | Lot creation was outside the rate-limited group and uncapped; on testnet a free demo key could create lots in a loop, each deriving an address or a wallet account. | `POST /auctions` is rate-limited like the other derive-an-address endpoints, and a key may hold at most 20 open lots (5 for a demo key). |
| 4 | medium | Email opt-in subscribed any address at once, so a stranger could make the operator's mail server send lot updates to a third party. | Double opt-in: a subscription sends one confirmation mail with a link; nothing else goes out until it is confirmed (`/notify/confirm`). At most three unconfirmed requests per address. |
| 5 | medium | The freeze check ignored a sealed lot's reveal window, so a lot could still be revealing when the service parked. | `lot_fits` adds `reveal_secs` before the settlement window. Test added. |
| 6 | low | `recipientTransparent` and `refundTo` went into upstream URLs unvalidated and unencoded. | Validated per chain and URL-encoded. |
| 7 | low | The status route passed a caller-supplied id into the Maya node's path. | Rail and id shapes validated; id encoded. |
| 8 | low | A signer's `/health` listed its policy and spend budget to anyone. | Only `{ok: true}` without the token; details with it. |
| 9 | low | The bid key was sent as a query string to the standing and fulfilment endpoints, where proxies and browsers log it. | Both are `POST` with a JSON body; the web client follows. |
| 10 | low | Upstream error bodies were relayed to clients. | Fixed messages; the body goes to the server log. |

Found sound and left alone: constant-time token comparison; admin and ledger token separation; parameterised
SQL throughout; no header injection through lettre; purge scope; envelope binding by address; the order of signer
checks (a refused session never consumes rolling budget); bid-card import keeps the secret client-side; CORS.

Still open, by design for the hackathon build: the rate limiter and signer session state are in memory (one
instance each); there is no account lockout on seller keys beyond the per-IP limit.
