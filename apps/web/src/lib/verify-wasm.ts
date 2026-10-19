/** Loads the Blind Bazaar verifier WebAssembly module from /wasm at runtime (built by scripts/build-wasm.sh). */

export interface VerifyWasm {
  compute_cmx(receiverHex: string, valueZats: bigint, rhoHex: string, rseedHex: string, noteVersion: number): string;
  decrypt_note(keyHex: string, encCiphertextHex: string): string;
  /** Receiver wallet from 32 bytes of entropy; see packages/bb-verify-wasm. */
  receiver_from_entropy(network: "main" | "test", entropyHex: string, transparentPkhHex: string): string;
}

let loading: Promise<VerifyWasm> | null = null;

export function loadVerifier(): Promise<VerifyWasm> {
  if (!loading) {
    loading = (async () => {
      const url = "/wasm/bb_verify_wasm.js";
      const mod = (await import(/* webpackIgnore: true */ url)) as {
        default: (opts?: { module_or_path?: string }) => Promise<unknown>;
        compute_cmx: VerifyWasm["compute_cmx"];
        decrypt_note: VerifyWasm["decrypt_note"];
        receiver_from_entropy: VerifyWasm["receiver_from_entropy"];
      };
      await mod.default({ module_or_path: "/wasm/bb_verify_wasm_bg.wasm" });
      return { compute_cmx: mod.compute_cmx, decrypt_note: mod.decrypt_note, receiver_from_entropy: mod.receiver_from_entropy };
    })();
  }
  return loading;
}
