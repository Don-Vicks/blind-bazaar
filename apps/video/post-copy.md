# Post copy

## Tweet

Blind Bazaar: a sealed-bid auction house on Zcash. A bid is one shielded payment with the bid in its encrypted memo; the chain closes the lot; losers are refunded privately; the winner is proven with a note-opening disclosure. One full lot ran on testnet, start to finish. Built for ZECATHON. github.com/Don-Vicks/blind-bazaar

## Forum post (one paragraph)

We built Blind Bazaar for ZECATHON: a sealed-bid auction house where every bid is a shielded Zcash payment carrying the bid inside its encrypted memo, with no smart contract. A lot is a fresh Orchard address and a viewing key; the first block after the close time decides what was late; losing deposits come back as shielded payments after a random delay; the winning payment is proven with a note-opening disclosure anyone can check against a public lightwalletd. Four formats run on the same rails (first price, editions, Vickrey with the second price proven from a Merkle path, request for quotes), sealed-reveal lots keep bids from the operator until reveal, and an audit tool recomputes a lot from the chain. Go Dark is the front door: a pay button that takes SOL, USDC or ETH into shielded ZEC, and a recipient with no wallet can make one in the browser. What is real and what is not is in the README's honesty table: one lot (jck1vba7) has run the whole loop on the public testnet with bids from an independent wallet; on classic lots the operator reads bids while the lot is open, and sealed-reveal lots remove that; the FROST 2-of-3 escrow and batched refunds are unit-tested but have not yet signed on a chain; no Go Dark swap has moved real money. Code, evidence and the two videos: github.com/Don-Vicks/blind-bazaar

## YouTube description

Blind Bazaar + Go Dark, built for ZECATHON 2026 (private markets track).

A sealed-bid auction house on Zcash. Every bid is a shielded transaction whose encrypted memo carries the bid. No contract: the lot is a fresh shielded address and a viewing key. The chain closes the lot, not our clock. Losers are refunded privately after a random delay. The winner is proven with a note-opening disclosure that anyone can recompute against the chain.

What you see in this video is a real lot on the public Zcash testnet (jck1vba7): two bids from an independent wallet, the close at block 4,530,003, the result, the verifier, the settlement, the seller payout and the refund. The transaction ids are in docs/evidence/testnet-jck1vba7/txids.md.

Go Dark is a pay button for any site: a payer on Solana or Ethereum lands in shielded ZEC, and a recipient with no Zcash wallet can make one in the browser. Quotes are live; no mainnet swap has been exercised in this build.

Plainly: on classic lots the operator reads bids while a lot is open; sealed-reveal lots remove that. The FROST 2-of-3 escrow and batched refunds are built and unit-tested but have not yet signed on a chain. Only the bundled command-line wallet has placed a bid.

Code and evidence: https://github.com/Don-Vicks/blind-bazaar
Write-up: docs/WRITEUP.md
Honesty table: README.md

No music; captions on screen.
