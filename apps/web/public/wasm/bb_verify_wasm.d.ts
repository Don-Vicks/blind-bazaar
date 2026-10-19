/* tslint:disable */
/* eslint-disable */

/**
 * Recompute the extracted note commitment (cmx) for a disclosed opening. Returns lowercase hex.
 */
export function compute_cmx(orchard_receiver_hex: string, value_zats: bigint, rho_hex: string, rseed_hex: string, note_version: number): string;

/**
 * Decrypt one Orchard note ciphertext (580 bytes, hex) with a disclosed 32-byte key. Returns JSON
 * `{version, diversifier, value, rseed, memo}`; `memo` is the UTF-8 text memo or null.
 */
export function decrypt_note(key_hex: string, enc_ciphertext_hex: string): string;

/**
 * A receiver wallet made in the browser: the same ZIP-32 derivation the service and the mobile wallets use
 * (Orchard spending key from the 24-word phrase's seed, coin type 133 on mainnet, account 0, diversifier 0),
 * so the phrase imports into Zodl, Zingo or YWallet unchanged. Entropy comes from the caller
 * (`crypto.getRandomValues`, 32 bytes hex). The transparent receiver is derived by the caller (BIP 44
 * `m/44'/133'/0'/0/0`; secp256k1 does not build for wasm here) and passed in as its 20-byte public key hash;
 * this function assembles the unified address from the Orchard receiver and that hash. Nothing here keeps
 * state or touches the network. Returns JSON `{phrase, seed, address, orchard_only}`.
 */
export function receiver_from_entropy(network: string, entropy_hex: string, transparent_pkh_hex: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly compute_cmx: (a: number, b: number, c: bigint, d: number, e: number, f: number, g: number, h: number) => [number, number, number, number];
    readonly decrypt_note: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly receiver_from_entropy: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
