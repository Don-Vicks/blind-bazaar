/** Bid-ledger inclusion proofs. Mirrors services/auction/src/merkle.rs exactly. */

export interface Sibling {
  hash: string;
  side: "left" | "right";
}

export interface InclusionProof {
  leaf: string;
  index: number;
  leaf_count: number;
  path: Sibling[];
  root: string;
}

const enc = new TextEncoder();

function hex(b: ArrayBuffer | Uint8Array): string {
  return [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
}

function unhex(s: string): Uint8Array {
  if (!/^[0-9a-f]*$/.test(s) || s.length % 2) throw new Error("bad hex");
  return Uint8Array.from(s.match(/../g) ?? [], (h) => parseInt(h, 16));
}

async function sha256(...parts: Uint8Array[]): Promise<Uint8Array> {
  const total = parts.reduce((n, p) => n + p.length, 0);
  const buf = new Uint8Array(total);
  let o = 0;
  for (const p of parts) {
    buf.set(p, o);
    o += p.length;
  }
  return new Uint8Array(await crypto.subtle.digest("SHA-256", buf));
}

/** Leaf = sha256(0x00 || receipt || bid_be8 || nonce). The nonce blinds the amount from everyone but the bidder. */
export async function leafHash(receipt: string, bidZats: bigint, nonce: string): Promise<string> {
  const be = new Uint8Array(8);
  new DataView(be.buffer).setBigUint64(0, bidZats);
  return hex(await sha256(new Uint8Array([0]), enc.encode(receipt), be, enc.encode(nonce)));
}

/** Recompute the root from your receipt, bid and nonce and the server-supplied path; true if it equals `proof.root`. */
export async function verifyInclusion(receipt: string, bidZats: bigint, nonce: string, proof: InclusionProof): Promise<boolean> {
  let acc = unhex(await leafHash(receipt, bidZats, nonce));
  if (hex(acc) !== proof.leaf) return false;
  for (const s of proof.path) {
    const sib = unhex(s.hash);
    acc = s.side === "right" ? await sha256(new Uint8Array([1]), acc, sib) : await sha256(new Uint8Array([1]), sib, acc);
  }
  return hex(acc) === proof.root;
}
