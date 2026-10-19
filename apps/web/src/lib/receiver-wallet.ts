/** A Zcash receiver wallet made in this browser, for people who want to receive shielded ZEC and have no wallet
 *  yet. The 24 words are standard BIP 39; the shielded key is the standard ZIP 32 Orchard derivation (account 0),
 *  computed in WebAssembly by the same code path the service uses; the transparent receiver is BIP 44
 *  m/44'/133'/0'/0/0, computed here with audited curve code. Nothing leaves the page: no network call, no
 *  storage. The phrase imports into Zodl, Zingo or YWallet unchanged. */

import { secp256k1 } from "@noble/curves/secp256k1.js";
import { hmac } from "@noble/hashes/hmac.js";
import { ripemd160 } from "@noble/hashes/legacy.js";
import { sha256, sha512 } from "@noble/hashes/sha2.js";
import { bytesToHex, hexToBytes } from "@noble/hashes/utils.js";
import { loadVerifier } from "./verify-wasm";

export interface ReceiverWallet {
  network: "main" | "test";
  phrase: string;
  /** Unified address with an Orchard receiver and a transparent receiver (so every rail can pay it). */
  address: string;
  /** The same key's Orchard-only unified address, as lot addresses are. */
  orchardOnly: string;
  /** The transparent receiver alone, for rails that refuse unified addresses. */
  transparent: string;
}

const N = secp256k1.Point.CURVE().n;
const HARD = 0x80000000;

function ser32(i: number): Uint8Array {
  const b = new Uint8Array(4);
  new DataView(b.buffer).setUint32(0, i >>> 0);
  return b;
}

function concat(...parts: Uint8Array[]): Uint8Array {
  const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
  let o = 0;
  for (const p of parts) {
    out.set(p, o);
    o += p.length;
  }
  return out;
}

function bytesToBigint(b: Uint8Array): bigint {
  return BigInt("0x" + bytesToHex(b));
}

function bigintTo32(n: bigint): Uint8Array {
  return hexToBytes(n.toString(16).padStart(64, "0"));
}

/** BIP 32 child key derivation from a seed along a path; returns the leaf private key. */
function bip32PrivateKey(seed: Uint8Array, path: number[]): Uint8Array {
  const master = hmac(sha512, new TextEncoder().encode("Bitcoin seed"), seed);
  let k: Uint8Array = master.slice(0, 32);
  let chain: Uint8Array = master.slice(32);
  for (const index of path) {
    const data =
      index >= HARD ? concat(new Uint8Array([0]), k, ser32(index)) : concat(secp256k1.getPublicKey(k, true), ser32(index));
    const I = hmac(sha512, chain, data);
    const child = (bytesToBigint(I.slice(0, 32)) + bytesToBigint(k)) % N;
    if (child === 0n) throw new Error("bip32: zero child key");
    k = bigintTo32(child);
    chain = I.slice(32);
  }
  return k;
}

/** Transparent receiver public key hash for the wallet's first external address. */
export function transparentPkh(seed: Uint8Array, coinType: number): Uint8Array {
  const key = bip32PrivateKey(seed, [HARD + 44, HARD + coinType, HARD + 0, 0, 0]);
  return ripemd160(sha256(secp256k1.getPublicKey(key, true)));
}

const BASE58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

function base58check(payload: Uint8Array): string {
  const check = sha256(sha256(payload)).slice(0, 4);
  const bytes = concat(payload, check);
  let n = bytesToBigint(bytes);
  let out = "";
  while (n > 0n) {
    out = BASE58[Number(n % 58n)] + out;
    n /= 58n;
  }
  for (const b of bytes) {
    if (b !== 0) break;
    out = "1" + out;
  }
  return out;
}

/** Encode a public key hash as a Zcash transparent address (t1… on mainnet, tm… on testnet). */
export function transparentAddress(pkh: Uint8Array, network: "main" | "test"): string {
  const prefix = network === "main" ? [0x1c, 0xb8] : [0x1d, 0x25];
  return base58check(concat(new Uint8Array(prefix), pkh));
}

export async function generateReceiverWallet(network: "main" | "test"): Promise<ReceiverWallet> {
  const entropy = new Uint8Array(32);
  crypto.getRandomValues(entropy);
  const wasm = await loadVerifier();
  // Two passes: the first yields the seed (so the transparent key can be derived here), the second assembles the
  // unified address with the transparent receiver. The entropy, and so the phrase, is the same in both.
  const first = JSON.parse(wasm.receiver_from_entropy(network, bytesToHex(entropy), "")) as { phrase: string; seed: string; orchard_only: string };
  const pkh = transparentPkh(hexToBytes(first.seed), network === "main" ? 133 : 1);
  const full = JSON.parse(wasm.receiver_from_entropy(network, bytesToHex(entropy), bytesToHex(pkh))) as { phrase: string; address: string; orchard_only: string };
  if (full.phrase !== first.phrase || full.orchard_only !== first.orchard_only) throw new Error("derivation mismatch");
  entropy.fill(0);
  return { network, phrase: full.phrase, address: full.address, orchardOnly: full.orchard_only, transparent: transparentAddress(pkh, network) };
}
