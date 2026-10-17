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
   itself, recomputes the shielded sighash, reads every unsigned spend's `alpha`, and sums the outputs
   it can see: change (recipients owned by the escrow viewing key) versus external value. It refuses
   anything over its `--max-external-zats`, logs the summary, generates fresh nonces per spend, and
   returns commitments.
3. Spender: with `threshold` commitments in hand, builds one signing package per spend
   (sighash + all commitments) and sends them (`POST /frost/round2 {session, packages}`).
4. Signer: checks each package's message is the session's sighash, signs with the randomizer `alpha`
   of that spend (`frost_rerandomized::sign`), and **deletes the session**: nonces are single-use
   whatever happens.
5. Spender: aggregates each signature, checks `ak + [alpha]G` equals the action's `rk`, and applies it
   through `pczt::roles::signer::Signer::apply_orchard_spend_auth_signature`, which verifies the
   signature under `rk` before storing it. Then spend-finalize, extract, store, broadcast.

Signers that are unreachable are skipped; the session fails if fewer than `threshold` commit. The
event log records which signers co-signed each payout (`cosigners` on `refund_sent`, `seller_paid`).

## Trust model

- A compromised spender host cannot move funds alone. It can propose transactions; the signers see
  the real destinations and amounts from the PCZT and apply their policy.
- A single compromised signer cannot move funds, and cannot forge a session: FROST shares are useless
  below threshold, and the spender verifies every signature under the on-chain `rk`.
- The viewing key is still shared by the viewer, spender and signers. Privacy of bidders against the
  operator set is unchanged (see `docs/PRIVACY.md`); what changed is that theft needs collusion.
- Trusted-dealer keygen means one machine briefly saw all shares. Run it air-gapped and discard the
  dealer state; distributed key generation is the follow-up (reddsa ships `frost::keys::dkg`).

## Running it

```
bb-auction frost keygen --out ./frost
# host A, B, C (each gets one share and the public escrow file)
bb-auction frost signer --share signer-1.json --escrow escrow.json --bind 0.0.0.0:8801 --max-external-zats 100000000
# spender
FROST_ESCROW=./frost/escrow.json FROST_SIGNERS=https://a:8801,https://b:8801,https://c:8801 FROST_TOKEN=… ROLE=spender bb-auction
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
