# Demo script (3 minutes)

One sealed lot on a public chain, start to finish: two wallet bids and one Go Dark entry, the chain closes it, the
result is proven, losers are refunded, the seller is paid. Everything shown exists today; the lines say what is
private and what is not.

## Timing facts the script must respect

- Zcash blocks are 75 seconds on average. A bid counts once it is mined and scanned, so bid at least 10 minutes
  before close.
- The chain, not the clock, closes the lot: the service waits for a block timestamped after `close_at`. Expect the
  result 30 to 180 seconds after the countdown hits zero.
- Refunds leave after a random delay (30 to 300 seconds in production; set 5 to 15 for the recording and say so).
- A Go Dark swap takes 2 to 10 minutes. It is pre-staged, never done live.

## Lot settings

- `format: vickrey`, `units: 1`, reserve = deposit = 0.05 ZEC. Bidder A and bidder C bid exactly the reserve;
  bidder B (via Go Dark) bids more. The second price is then the reserve, which the deposit already covers, so the
  winner owes nothing at close and the lot settles itself: seller payout about 30 seconds after close, B's overshoot
  refunded automatically. No live settlement transaction is needed.
- Create the lot at least 45 minutes before recording with an absolute `close_at`. Stage three lots with
  `close_at` six minutes apart so one closes inside the take; keep the raw recordings and link them.
- Recording-only settings: `SYNC_INTERVAL_SECS=5`, `REFUND_DELAY_MIN_SECS=5`, `REFUND_DELAY_MAX_SECS=15`,
  `REFUND_WINDOW_SECS=10`. Say on screen that production uses minutes to hours and that at demo density a refund
  is linkable by timing (`docs/evidence/refund-linkability.md`).

## Take (close_at = 0:35 of the take)

| Time | On screen | Line |
|---|---|---|
| 0:00 | Title card | "Every auction on Ethereum publishes every bid forever. This one publishes the winner and a proof, and nothing about the losers." |
| 0:08 | Lot page: countdown, three receipts, "listed 45 min ago" | "No contract. The lot is a fresh shielded address. Each bid is one private payment with the bid inside its encrypted memo." |
| 0:20 | Bidder B's finished Go Dark tab: Solana signature, Zcash delivery txid, receipt | "B had no ZEC. Go Dark swapped USDC from Phantom into shielded ZEC at the lot's address and relayed the sealed bid. The rail saw B's Solana address; it never saw the bid." |
| 0:35 | Countdown hits zero; explorer tab showing the latest block | "The chain closes it, not our clock. A bid mined after this block is late and refunded." |
| wait | `/embed` builder while the block lands (cut here if long; show the UTC clock and block height) | "The same button is a private pay button for any site." |
| close + 5 s | Result page: price, ledger root, close height, price proof "The second bid is in the ledger" | "B wins and pays the second-highest bid. The second price is proven by a Merkle path into the bid ledger committed at close. The winning bid itself stays sealed, even from the seller." |
| close + 25 s | `/verify` in the browser, then the CLI verifier against a public lightwalletd prints OK | "Anyone can rebuild the winning payment's commitment from the published opening and check it against the chain. Nothing about any loser is revealed." |
| close + 50 s | Standing panels: A "Not this time", refund queued; seller dashboard: payout sent | "Losers get their deposit back as a private payment after a random delay. The seller is paid. On the chain these are ordinary shielded transactions." |
| 2:35 | README honesty table | "What is private, what is real, what is not: the operator reads bids while a lot is open, and the Solana rail may deliver transparently. We say so." |
| 2:50 | Repo, evidence folder, tracks | End |

## Pre-flight

- Public HTTPS deployment, `/health` lag 0 (admin view: float covers three refunds plus fees, in separate notes with
  10+ confirmations).
- Wallets: A (Zodl or Zingo, funded), C (a second wallet), B (Phantom with USDC and SOL for gas), a seller wallet
  and a refund address for B.
- Seller key created; three lots created with staggered `close_at`.
- B's swap completed and its receipt visible on the status page; A and C mined at least 10 minutes before close.
- Verifier binary built; browser tabs open: lot, status, result, verify, explorer, dashboard.
- Backup recording from the dry run saved.

## What to say plainly if asked

- The operator's viewing key reads bids while a lot is open; hiding them from the operator too is on the roadmap.
- The Solana rail may deliver to a transparent receiver; that bid's amount is then visible for one hop before
  auto-shielding.
- The FROST escrow is built and unit-tested; the demo instance runs the single-key configuration unless the
  escrow chain run has landed.
