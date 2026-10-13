# Landscape: Arcium, comparable hackathons, and what it means for us

Research notes, Oct 13, 2026. Sources at the end. Claims about third parties are as reported by those sources.

## Arcium in one paragraph

Arcium is Solana's MPC network (formerly Elusiv). Mainnet Alpha since Feb 2026, permissioned: four node operators run
by or selected by Arcium. Developers write "confidential instructions" in Arcis (a Rust DSL) inside an Anchor program
(an MXE); clients encrypt inputs with x25519 + RescueCipher to the MXE key; the cluster computes and signs the
output, which a callback verifies on chain. The live protocol, Cerberus, keeps inputs private as long as at least one
node is honest, and aborts on tampering. Fees are paid in SOL. Limits: Solana only, fixed-shape circuits, outputs
under 1,232 bytes, no cross-chain input.

Arcium's own `sealed_bid_auction` example (and the community BlindBid) encrypts bids to the MXE, keeps a running
top-two inside MPC, and reveals only winner and clearing price. It has no deposits, no escrow, no settlement, does not
bind the encrypted bidder to the signer, and does not enforce the minimum bid in-circuit. Crafts (uniform-price
sealed-bid token launches) is the production app in this niche and won "best integration" at the Solana Privacy
Hack.

**Versus Blind Bazaar.** Arcium removes the one party who can read bids before close; we currently have that party
(the viewer). Arcium leaks bidder public keys and the result on a transparent chain and has no money privacy; we
have shielded deposits, refunds and settlement end to end, and cross-chain entry. The two are complementary: MPC for
*who wins*, Zcash for *where the money goes*.

## What won at comparable hackathons

- **Colosseum Cypherpunk (2025)**: a startup competition, no privacy track. The only privacy placement was Cloak
  (shielded pool on Solana, mainnet alpha, SDK, viewing keys for selective disclosure). Judges score team,
  functionality, traction, novelty, UX, open source; the 3-minute pitch video decides shortlisting.
- **Solana Privacy Hack (Jan 2026)**: Arcium track went to hidden-information *markets* (Bench, Crafts sealed-bid
  uniform-price auction, Epoch prediction market, Undesk OTC with relayer-derived keys). Rules: open source, devnet or
  mainnet, video under 3 minutes, run docs. Range's compliance bounty rewarded privacy *with* selective disclosure.
- **ZecHub 2.0 and 3.0**: mainnet required. Winners used Zcash primitives deeply: encrypted memo as a protocol channel
  (ZcashMe OTP-in-memo, ZECA relay), UFVK watch-only (Pendrake), FROST escrow (zk.poker, CYZE), ZIP-321. Testnet-only
  entries did not place.
- **Zcash x NEAR Intents (2025)**: judges withheld second and third prizes for low quality; a complete working
  cross-chain-to-shielded flow was itself rare.
- **Zypherpunk (Dec 2025)**: ECC's biggest prizes went to SDKs that move funds into the shielded pool (t2z); the NEAR
  bounty went to the one project chaining Intents, Solana and Zcash end to end. ZIntents (scheduled, amount-randomised
  shielded withdrawals) is the closest prior art to Go Dark.

## The organiser's own auction

zkSNARKs ran a 48-hour uniform-price sealed-bid auction in Sept 2026: bids as shielded ZEC transactions carrying a
max price, 16,971 bids, everyone paid the lowest winning bid, 13,309 ZEC refunded by shielded transactions within 24
hours, over half the volume entered through NEAR Intents from Solana USDC and ETH. Its documented weaknesses: results
were self-reported with no public verification, the source-chain leg of cross-chain entry was public, and refunds
depended on trusting the operator. The event drew public criticism afterwards. The judges have lived exactly the
problem we are solving; a submission that fixes those three points speaks to them directly.

## What we change

Ordered by payoff against the Oct 28 deadline.

1. **Mainnet auction plus a public verify page.** One completed lot on mainnet with the winning bid, the note-opening
   proof and a one-command verifier. "Provable results, shielded bids" is the headline.
2. **Uniform-price multi-unit mode.** N identical units, top-N win, all pay the lowest winning bid, losers refunded,
   winners refunded the difference. This is the zkSNARKs format and the Crafts format; pitch it as that drop rebuilt
   with provable results and automatic refunds.
3. **Refund latency as a metric.** Shielded refunds inside a short window of close, shown on the result page.
4. **Go Dark as the bid entry path with an honest threat model** and source-leg mitigations: per-payment addresses,
   auto-shield on arrival where a rail delivers transparent, optional delayed or amount-randomised delivery.
5. **Go Dark as an SDK with a second integrator** (a checkout or domain purchase flow), published to npm.
6. **One more native primitive, finished:** viewing-key selective disclosure so a seller or auditor can prove the
   reserve was met or verify the winner set without unmasking losers.
7. **Arcium MPC mode for winner selection** (devnet cluster 456): bids encrypted to an MXE, Vickrey or uniform
   clearing inside MPC, winner sealed to the seller, Zcash deposits with a uniform fixed amount carrying the Solana
   receipt hash. Removes the viewer's ability to read bids. Stretch for the hackathon, first item after.
8. **Submission package to the strictest rulebook:** MIT, exact run and test steps, architecture diagram, memo spec,
   threat model, a pitch video under 3 minutes and a separate technical walkthrough, public building before the
   sealed submission for Community Choice.

## Sources

Arcium docs and examples: https://docs.arcium.com , https://github.com/arcium-hq/examples ,
https://rtg.arcium.com/rtg/dev-blind-auctions , https://arcium.substack.com/p/winners-of-solana-privacy-hack ,
https://github.com/addnad/blindbid . Colosseum: https://blog.colosseum.com/announcing-the-winners-of-the-solana-cypherpunk-hackathon/ ,
https://blog.colosseum.com/how-to-win-a-colosseum-hackathon/ . Solana Privacy Hack: https://solana.com/privacyhack ,
https://www.inco.org/blog/solana-privacy-hack-inco-prize-winners , https://range.org/blog/privacy-meets-compliance-the-winners-of-our-solana-privacy-hack-bounty .
ZecHub: https://zechub.substack.com/p/announcing-the-winners-of-zechub , https://zechub.substack.com/p/zechub-hackathon-results-are-in .
Zypherpunk: https://forum.zcashcommunity.com/t/zypherpunk-hackathon-winners/53985 , https://devfolio.co/projects/zintents-a311 .
zkSNARKs auction: https://cryptobriefing.com/zksnarks-auction-clears-zcash-blind-bids/ ,
https://crypto.news/aurora-intents-routed-19m-into-zcash-nft-auction/ , https://cryptobriefing.com/zksnarks-nft-17m-extraction-allegations/ .
ZECATHON: https://thezecathon.com/ , https://x.com/zksnarks_/status/2104606505177899314 .
