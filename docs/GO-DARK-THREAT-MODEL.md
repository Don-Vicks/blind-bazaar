# Go Dark threat model

What each party can see when someone enters Blind Bazaar, or pays a site, through Go Dark. Written so a judge or an
integrator can check every claim against the code.

## The hops

```
payer wallet (Solana or Ethereum)  →  swap rail (NEAR Intents or Maya)  →  Zcash unified address  →  auction service
```

| Hop | What is visible, and to whom | What is hidden |
|---|---|---|
| Payer wallet → rail | **Public.** The source chain records the payer's address, the amount, and the rail's deposit address. Anyone can see that this wallet sent, say, 12.3 USDC to a NEAR Intents deposit address at this time. | Nothing on this hop. This is the leak the organiser's own auction documented. |
| Inside the rail | The rail operator sees the payer's source address, the amount, and the Zcash destination address. NEAR Intents screens payers against sanctions lists. | The bid. It never travels through the rail. |
| Rail → Zcash | One shielded output lands at the destination. A chain observer sees a shielded transaction of unknown amount and recipient. | Amount, recipient, and any link to the auction. |
| Zcash → auction service | The service, holding the viewing key, decrypts the note: amount and the fact that a deposit arrived at a lot's address. The sealed bid arrives separately as an envelope over HTTPS keyed by the rail's deposit address. | Nothing further from the service's point of view: it is the trusted reader in both the memo path and the envelope path. |
| Auction service → public | Receipt hashes, the ledger commitment, the result, the note-opening proof of winning outputs. | Losing bids and bidders. |

## Mitigations in the code

- **Fresh address per payment.** The pay-privately mode asks the service for a new diversified unified address for
  every payment (`GET /godark/receive-address`), so two payments to the same site cannot be linked by destination.
  Auction entries go to the lot's own fresh address.
- **Shielded delivery first.** Both rails are asked to deliver to the unified address. Maya accepts it. NEAR Intents
  accepts it at quote time; if a rail rejects it, the proxy falls back to the transparent receiver of that same
  address and tells the widget, which then says so in its "who sees what" panel.
- **Auto-shield on arrival.** Any funds that land on a transparent receiver are moved into the shielded pool by the
  spender process on the next pass, before the auction logic looks at them. Transparent deliveries are matched to
  bids by the rail's destination txid, not by memo.
- **No logging of addresses.** The quote proxy logs status codes and timing only. The service never stores the
  payer's source address; the envelope carries the rail's deposit address, which is a one-time account.
- **Rate limit and origin allow-list on the proxy** so the widget cannot be used to enumerate quotes or addresses.

## What we do not hide, said plainly

- The source-chain transfer. If your Solana or Ethereum wallet is known to be yours, the fact that you sent funds
  to a swap rail at a given time is public. Use a fresh source wallet if that matters.
- Timing. A swap that completes just before a bid lands is a correlation signal to someone watching both the source
  chain and the shielded pool size. Refunds after close are delayed by a random interval for the same reason.
- The rail's own records. NEAR Intents and Maya know who paid and where it went. Choose the rail accordingly.
- Amount granularity. Deposits are fixed per lot precisely so the amount leaks nothing about the bid.

## Not implemented, and why

- Scheduled or amount-randomised delivery through the rail: the rails deliver when they deliver. The service could
  hold and forward, but that would put it in custody of funds that are not yet a bid. Deferred.
- Deposit from a fresh derived source address: that is a wallet feature, not something an embedded widget can do
  safely. The widget shows the advice instead.
