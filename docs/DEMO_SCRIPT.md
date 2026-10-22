# Demo script (3 minutes, Zcash testnet)

One sealed lot on the public testnet, start to finish: two bids from an independent wallet, the chain closes
it, the result is proven, the loser is refunded, the winner settles, the seller is paid. Everything shown exists
today; the lines say what is private and what is not. There is no mainnet money in this build, so Go Dark is
shown with live quotes and the browser wallet, not a wet swap, and the script says so.

## Timing facts the script must respect

- Testnet blocks are 75 seconds on average. A bid counts after two confirmations, so bid at least 6 minutes
  before close.
- The chain, not the clock, closes the lot: the service waits for a block timestamped after `close_at`. Expect the
  result 30 to 180 seconds after the countdown hits zero.
- Refunds leave after a random delay (30 to 300 seconds by default; set 5 to 15 for the recording and say so).
- The winner's settlement is a second payment; from the bundled wallet it is one command and one block.

## Lot settings

- `format: first_price`, reserve 0.02 TAZ, deposit 0.01 TAZ (both on the standard menu), bidder A bids 0.03,
  bidder B bids 0.025. Seller address from a third wallet.
- Create the lot 20 minutes before recording with an absolute `close_at`. Stage three lots six minutes apart so
  one closes inside the take; keep the raw recordings and link them.
- Recording-only settings on the instance: `SYNC_INTERVAL_SECS=5`, `REFUND_DELAY_MIN_SECS=5`,
  `REFUND_DELAY_MAX_SECS=15`, `REFUND_WINDOW_SECS=10`. Say on screen that production uses minutes to hours and
  that at demo density a refund is linkable by timing (`docs/evidence/refund-linkability.md`).
- The bids: `bb-auction wallet --dir ./bidder --seed-file ./bidder.seed --pay "<uri from the lot page>"`, run
  twice with two bid keys. Show the terminal: one shielded payment per bid, the memo never printed.

## Take (close_at = 0:35 of the take)

| Time | On screen | Line |
|---|---|---|
| 0:00 | Title card | "Every auction on Ethereum publishes every bid forever. This one publishes the winner and a proof, and nothing about the losers." |
| 0:08 | Lot page: countdown, two receipts, "listed 20 min ago" | "No contract. The lot is a fresh shielded address. Each bid is one private payment with the bid inside its encrypted memo. These two came from a wallet that is not ours." |
| 0:20 | Testnet explorer: one of the bid txids | "On the chain it is an ordinary shielded transaction: no amount, no sender, no memo." |
| 0:35 | Countdown hits zero; explorer tab showing the latest block | "The chain closes it, not our clock. A bid mined after this block is late and refunded." |
| wait | `/embed`: the pay button builder, a live Solana quote, then "Make a wallet in this browser" | "The same rails are a private pay button for any site. A payer on Solana gets a live quote into shielded ZEC; a recipient with no wallet makes one here, nothing leaves the page. Mainnet rails are not exercised in this take." |
| close + 5 s | Result page: price, ledger root, close height, winner's disclosure | "A wins and pays 0.03. The ledger root was committed at close; the losing bid stays sealed, even from the seller." |
| close + 25 s | `/a/ID/verify` in the browser, then the CLI verifier against a public lightwalletd prints OK | "Anyone can rebuild the winning payment's commitment from the published opening and check it against the chain. Nothing about the loser is revealed." |
| close + 50 s | Terminal: the winner pays the settlement URI; standing panel: B "Not this time", refund sent; seller dashboard: payout sent | "The loser's deposit comes back as a private payment after a random delay. The winner pays the balance; the seller is paid. On the chain these are ordinary shielded transactions." |
| 2:35 | README honesty table | "What is private, what is real, what is not: on classic lots the operator reads bids while a lot is open; sealed-reveal lots remove that. We say so." |
| 2:50 | Repo, evidence folder, tracks | End |

## Pre-flight

- `scripts/preflight.sh https://your.domain` all green; `/health` lag 0; operator float at least 0.1 TAZ.
- Wallets: a bidder wallet with 0.1 TAZ (fauzec.com, one drip per address per day), a seller wallet address.
- Three lots created with staggered `close_at`; bids mined at least 6 minutes before close.
- Verifier binary built; browser tabs open: lot, status, result, verify, explorer, dashboard, embed.
- Backup recording from the dry run saved.

## What to say plainly if asked

- The operator's viewing key reads bids while a classic lot is open; sealed-reveal lots remove that.
- Go Dark's rails are mainnet-only and no swap has moved real money in this build. Maya refuses Orchard-only
  unified addresses since Ironwood, so the Ethereum path delivers to a transparent receiver and is shielded on
  arrival; the widget says so.
- The FROST escrow has signed on a local regtest chain (`docs/evidence/regtest/`); the testnet instance runs
  the single-key configuration.
