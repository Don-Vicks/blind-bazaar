# FROST escrow: nobody holds the spending key

The spender is the process that refunds losers and pays sellers. Before this change it held a seed,
so whoever ran that host could drain the auction pool. Now the Orchard spend authority of the escrow
account is a **2-of-3 FROST group key** (RedPallas, re-randomized FROST as in ZIP 312 / reddsa) and
the spender holds only the viewing key. Every payout is a PCZT that two independent signer hosts
inspect and co-sign.

## Keys

| Key | Where | Purpose |
|---|---|---|
| `ak` (spend validating key) | group verifying key of the FROST shares | spend authority; never exists as a single scalar |
| `nk`, `rivk` | minted once at keygen, in `escrow.json` | nullifier derivation and `ivk` commitment |
| full viewing key `ak‖nk‖rivk` | spender and viewer wallets | scan, select inputs, build and prove transactions |
| signer shares | `signer-N.json`, one per host, mode 0600 | produce signature shares |

`bb-auction frost keygen --out ./frost` runs trusted-dealer generation (`reddsa::frost::redpallas`) and
writes the shares and the escrow description. `bb-auction frost show --escrow escrow.json` prints the
unified full viewing key and the first address; the viewer imports that key exactly as before.

The circuit does not learn `ask`: an Orchard action proves `rk = ak + [α]G` and the signature is
checked under `rk` outside the circuit, so a threshold `ak` needs no change to proving.

## One payout, step by step

1. Spender: wallet proposes inputs and change (`propose_transfer`), builds a PCZT
   (`create_pczt_from_proposal`) and proves it (`pczt::roles::prover`). Dummy spends are signed by the
   IO finalizer; real spends carry `alpha` and `rk` and no signature.
2. Spender → signers (`POST /frost/round1 {session, pczt}`): the whole PCZT, hex. The signer parses it
   itself, recomputes the shielded sighash, refuses any transparent or Sapling component, recomputes
   every action's value commitment and every output's note commitment from the recipient, value and
   randomness the PCZT states (so a spender cannot describe one payee and pay another), reads every
   unsigned spend's `alpha`, and sums the outputs: change (recipients owned by the escrow viewing key)
   versus external value, plus the fee. It refuses anything over its `--max-external-zats` or fee cap,
   logs the summary, generates fresh nonces per spend, and returns commitments.
3. Spender: with `threshold` commitments in hand, builds one signing package per spend
   (sighash + all commitments) and sends them (`POST /frost/round2 {session, packages}`).
4. Signer: checks each package's message is the session's sighash, requires exactly one package per
   planned spend (a repeated action with different co-signer commitments would leak the share through
   nonce reuse), signs with the randomizer `alpha` of that spend (`frost_rerandomized::sign`), and
   **deletes the session**: nonces are single-use whatever happens.
5. Spender: aggregates each signature, checks `ak + [alpha]G` equals the action's `rk`, and applies it
   through `pczt::roles::signer::Signer::apply_orchard_spend_auth_signature`, which verifies the
   signature under `rk` before storing it. Then spend-finalize, extract, store, broadcast.

Signers that are unreachable are skipped; the session fails if fewer than `threshold` commit. The
event log records which signers co-signed each payout (`cosigners` on `refund_sent`, `seller_paid`).

## The ledger check: signers pay only what the rules produced

With `--ledger-url` and `--ledger-token`, a signer fetches the viewer's `/ledger/payouts` before every session:
the list of payouts the viewer derived with the viewing key alone (refunds to the addresses sealed in losing
bids, seller payouts at the price, forfeits, and manual operator sends queued on the viewer). Every external
output in the PCZT must match one unconsumed entry by Orchard receiver and exact amount, and signing consumes
the entries it matched. A compromised spender can therefore neither invent a recipient nor pay a legitimate
one twice; a compromised viewer still cannot spend. The regtest script runs both signers with the check on.
Without the flag a signer enforces only the value and fee caps, and says so at startup.

## The rolling cap: a bound on what many small transactions can drain

`--max-external-zats` bounds one transaction. A spender that is compromised but still under the ledger check
cannot pay anyone the rules did not name; one that is compromised on a signer without the ledger check could
send many transactions each under the cap. `--max-external-per-window-zats` (with `--window-secs`, default one
day) refuses a session once the external value this signer has committed to inside the trailing window would
exceed the limit. A session counts from round 1, broadcast or not, so abandoned sessions cannot be used to park
budget. The window lives in the signer's memory (restarting the signer host clears it), which is one more reason
the signer hosts are machines the spender cannot touch. The signer's `/health` reports the limit and the value
used so far. A sensible setting is the operator float the lots of one day can need: deposits returned plus
seller payouts.

## Trust model

- A compromised spender host cannot move funds alone. It can propose transactions; the signers see
  the real destinations and amounts from the PCZT, and with the ledger check they sign only payments the
  viewer's ledger named.
- A single compromised signer cannot move funds, and cannot forge a session: FROST shares are useless
  below threshold, and the spender verifies every signature under the on-chain `rk`.
- The viewing key is still shared by the viewer, spender and signers. Privacy of bidders against the
  operator set is unchanged (see `docs/PRIVACY.md`); what changed is that theft needs collusion.
- Two ways to make the key. `frost keygen` is a trusted dealer: one machine briefly sees all shares, so
  run it air-gapped and discard the dealer state. `frost dkg` is distributed key generation: each host
  runs `round1`, `round2` and `finish` on its own machine and only public packages cross between hosts
  (round-2 packages go to one named peer over a private channel), so no machine ever holds more than
  its own share. One host then runs `frost dkg escrow` to mint `nk`/`rivk` and write `escrow.json`; the
  others check the `ak` it prints against their own share before using it.
- The escrow account is Orchard-only. A rail that can only deliver to a transparent address (NEAR
  Intents from Solana today) therefore cannot pay into the escrow; the widget refuses the quote in that
  configuration and says so. Solana entries need the seed-backed operator wallet, or a rail that delivers shielded.

## Running it

```
bb-auction frost keygen --out ./frost          # dealer, or:
bb-auction frost dkg round1 --id 1 --dir ./dkg  # on each host (id 1..3), then exchange files and run round2, finish, escrow
# host A, B, C (each gets one share and the public escrow file)
bb-auction frost signer --share signer-1.json --escrow escrow.json --bind 0.0.0.0:8801 --max-external-zats 100000000 --max-external-per-window-zats 500000000
# spender
FROST_ESCROW=./frost/escrow.json FROST_SIGNERS='https://a:8801|tokA,https://b:8801|tokB,https://c:8801|tokC' ROLE=spender bb-auction
# viewer (unchanged): OPERATOR_UFVK=<from `frost show`> ROLE=viewer bb-auction
```

`/health` reports `"signing": "frost 2-of-3 (…)"`. Each signer keeps an audit log of what it was
asked to authorise (`GET /frost/log`: session, sighash, external outputs and amounts, change, and
whether it committed, signed or refused). `FROST_SIGNER_SHARES` runs shares in-process for
development; it gives no separation and logs a warning. `scripts/regtest-e2e.sh` with `FROST=1`
runs the full money loop with two signer hosts up and the third deliberately absent.

## Implementation notes

- `packages/bb-frost`: reddsa 0.5.2 (`frost` feature) and `frost-rerandomized` 3. orchard 0.16 uses
  reddsa 0.6, which dropped the ciphersuite; the 32-byte key and 64-byte signature encodings are
  identical, so the crates exchange bytes. Tests prove a 2-of-3 signature verifies under `rk` and
  not under `ak`.
- `services/auction/src/frost.rs`: escrow file, signing plan from a PCZT (via
  `pczt::roles::low_level_signer`, which exposes each spend's `alpha`), wire protocol, signer host
  (axum), coordinator with HTTP and in-process transports. Four tests cover a 2-of-3 session with one
  signer offline, sub-threshold failure, policy refusal, and single-use sessions.
- `services/auction/src/wallet/pczt_send.rs`: the PCZT send path; `send.rs` dispatches on
  `Authority::{Seed, Frost}`.
