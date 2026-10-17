# Demo script (3 minutes)

One Solana user with no ZEC wins a blind auction; the losers are refunded; everything is shielded and the result
is provable. Record the full run twice. Keep the explorer tab visible throughout.

| Time | On screen | Line |
|---|---|---|
| 0:00 | Title card | "Every auction on Ethereum leaks every bid. Here is one where losing bids are never revealed and the result is provable." |
| 0:15 | Seller creates auction (curl or /sell), reserve 0.5 ZEC | "No contract deployed. The auction is a shielded address and a viewing key." |
| 0:35 | Bidder A scans the QR in Zodl | "The bid lives in the encrypted memo. Only the auction can read it." |
| 0:55 | Bidder B taps Enter with Go Dark, pays USDC from Phantom | "No ZEC, no Zcash wallet, one click. It lands shielded with the bid attached." |
| 1:25 | Bidder C bids; explorer shows three indistinguishable shielded txs | "Three bids. Try to tell them apart." |
| 1:45 | Countdown hits zero; result page shows winner | "Bidder B wins. A and C never learn what B paid or that each other existed." |
| 2:05 | Two refunds arrive in Zodl; winner pays balance; seller receives payout | "Refunds are shielded and delayed at random. Nothing on the chain links them to the auction." |
| 2:30 | Result page: disclosure JSON, then `--verify-disclosure` prints OK | "The result is provable. The losers are not." |
| 2:45 | /embed page with the snippet | "Any site can add this button. Every site that does onboards users into shielded Zcash." |
| 2:55 | Repo, tracks, team | End |

## Pre-flight

- Service `/health`: sync lag 0, balance covers refunds plus fees.
- Three wallets funded: Zodl (A), Phantom with USDC (B), Zingo or YWallet (C).
- Auction created with `duration_secs` around 240 so the close lands inside the recording.
- `REFUND_DELAY_MIN_SECS=20 REFUND_DELAY_MAX_SECS=60` for the recording only.
- Explorer tab open on the auction address's network.
- Backup recording from the previous dry run saved.
