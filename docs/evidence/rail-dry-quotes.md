# Go Dark rails: dry quotes, no money moved (Oct 19, 2026)

`node scripts/rail-dry-quotes.mjs` asks both mainnet rails for a quote to three recipients owned by one mainnet
key derived from the operator seed (`bb-auction derive --all-receivers` with `NETWORK=main`): the lot-style
unified address, the same key's unified address derived with every receiver, and its transparent receiver.
Nothing is sent. The same calls then went through the app's own proxy route (`POST /api/godark/quote`,
`dry: true`) on the dev server.

Dry quotes on 2026-10-19T14:56Z, wanting 0.01 ZEC

| Rail | Asset in | Recipient | Answer |
|---|---|---|---|
| Maya | ETH.ETH | u1 (Orchard only) | refused: ZecError: GenericError: something went wrong: Unified address has only an Orchard receiver; Orchard outputs are disabled from NU6.3 (ZIP 2006): u1c93m |
| Maya | ETH.USDC | u1 (Orchard only) | refused: ZecError: GenericError: something went wrong: Unified address has only an Orchard receiver; Orchard outputs are disabled from NU6.3 (ZIP 2006): u1c93m |
| 1Click | USDC (Solana) | u1 (Orchard only) | accepted (dry): 11.881882 in → 0.01 ZEC out, ~135s |
| 1Click | SOL | u1 (Orchard only) | accepted (dry): 0.113516288 in → 0.01 ZEC out, ~137s |
| Maya | ETH.ETH | t1 | accepted: 0.01 in → 0.02080 ZEC out, 24s, fee 184 bps, inbound 0xdc795fa8fe… |
| Maya | ETH.USDC | t1 | accepted: 100 in → 0.08761 ZEC out, 24s, fee 53 bps, inbound 0xdc795fa8fe… |
| 1Click | USDC (Solana) | t1 | accepted (dry): 11.864669 in → 0.01 ZEC out, ~137s |
| 1Click | SOL | t1 | accepted (dry): 0.11357382 in → 0.01 ZEC out, ~135s |

Maya did not quote to any unified address: Ethereum deliveries are transparent, shielded on arrival.

## What changed since the Oct 13 spike

- **Maya now refuses every unified address this service produces.** Its node answers
  `Unified address has only an Orchard receiver; Orchard outputs are disabled from NU6.3 (ZIP 2006)`. On Oct 13
  the same call was accepted (`docs/PRIVACY.md`). The key library this service uses says Ironwood shares the
  Orchard receiver, so the address is valid for Zcash wallets; Maya's integration does not treat it so. This key's unified address carries only an Orchard receiver (deriving it with every receiver allowed gives the
  same string); its transparent receiver is a separate `t1` address.
- **Consequence for Go Dark's Ethereum path:** delivery is transparent today. The proxy now tries the unified
  address, and when Maya refuses it, quotes to the transparent receiver and reports `delivery: "transparent"`;
  the widget then says the amount is visible for one hop and the service shields on arrival. Before this run
  the proxy returned a 502 (`Cannot convert undefined to a BigInt`) because Maya reports refusals with HTTP 200
  and an `error` field; that is fixed in `apps/web/src/lib/rails/maya.ts`.
- **1Click accepts a unified address at quote time**, for SOL and for Solana USDC, dry. Whether it delivers to
  the shielded receiver is still unknown until a wet swap; its docs say transparent-only.

## Through the app route (dev server, dry)

| Request | Answer |
|---|---|
| ethereum ETH, unified recipient plus transparent receiver | 200, `rail: maya`, `delivery: transparent`, memo `=:z:t1…`, 0.00502 ETH for 0.0103 ZEC, 24 s |
| ethereum ETH, unified recipient only | 502 with Maya's refusal text |
| solana USDC, unified recipient, dry | 200, `rail: oneclick`, `delivery: shielded`, 11.87 USDC for 0.01 ZEC, 137 s |

CI runs the script on every push as a non-blocking job so a change on either rail shows up in the log.
