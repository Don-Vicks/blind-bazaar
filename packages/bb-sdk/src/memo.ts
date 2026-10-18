/** BB1 memo format. Mirrors services/auction/src/memo.rs; see docs/MEMO_SPEC.md. */

export const MAX_MEMO_BYTES = 300;
const CROCKFORD = "0123456789abcdefghjkmnpqrstvwxyz";

export interface Bid {
  auctionId: string;
  bidZats: bigint;
  refundAddress: string;
  nonce: string;
}

export interface Settlement {
  auctionId: string;
  nonce: string;
}

/** A sealed-reveal bid: the memo carries sha256("<auction>|<bid_zats>|<salt>") instead of the amount. */
export interface Commit {
  auctionId: string;
  commitment: string;
  refundAddress: string;
  nonce: string;
}

export type Parsed = { kind: "bid"; bid: Bid } | { kind: "settlement"; settlement: Settlement } | { kind: "commit"; commit: Commit };

export class MemoError extends Error {
  constructor(public readonly code: string, message: string) {
    super(message);
    this.name = "MemoError";
  }
}

export function isAuctionId(s: string): boolean {
  return s.length === 8 && [...s].every((c) => CROCKFORD.includes(c));
}

export function isNonce(s: string): boolean {
  return /^[0-9a-f]{16}$/.test(s);
}

/** Loose shape check: unified address with a shielded receiver. Rails and wallets do full validation. */
export function looksLikeShieldedAddress(s: string): boolean {
  return (
    (s.startsWith("u1") || s.startsWith("utest1") || s.startsWith("uregtest1")) &&
    s.length > 20 &&
    /^[0-9a-z]+$/.test(s)
  );
}

export function parseMemo(memo: string): Parsed {
  if (memo.length > MAX_MEMO_BYTES) throw new MemoError("too_long", "memo too long");
  const trimmed = memo.replace(/\0+$/, "").trim();
  const [tag, ...fields] = trimmed.split("|");
  if (tag === "BB1") {
    if (fields.length !== 4) throw new MemoError("field_count", "wrong field count");
    const [auctionId, amount, refundAddress, nonce] = fields;
    if (!isAuctionId(auctionId)) throw new MemoError("auction_id", "invalid auction id");
    if (!/^\d+$/.test(amount) || BigInt(amount) === 0n) throw new MemoError("amount", "invalid bid amount");
    if (!looksLikeShieldedAddress(refundAddress)) throw new MemoError("refund_address", "invalid refund address");
    if (!isNonce(nonce)) throw new MemoError("nonce", "invalid nonce");
    return { kind: "bid", bid: { auctionId, bidZats: BigInt(amount), refundAddress, nonce } };
  }
  if (tag === "BB1C") {
    if (fields.length !== 4) throw new MemoError("field_count", "wrong field count");
    const [auctionId, commitment, refundAddress, nonce] = fields;
    if (!isAuctionId(auctionId)) throw new MemoError("auction_id", "invalid auction id");
    if (!isCommitment(commitment)) throw new MemoError("commitment", "invalid commitment");
    if (!looksLikeShieldedAddress(refundAddress)) throw new MemoError("refund_address", "invalid refund address");
    if (!isNonce(nonce)) throw new MemoError("nonce", "invalid nonce");
    return { kind: "commit", commit: { auctionId, commitment, refundAddress, nonce } };
  }
  if (tag === "BB1S") {
    if (fields.length !== 2) throw new MemoError("field_count", "wrong field count");
    const [auctionId, nonce] = fields;
    if (!isAuctionId(auctionId)) throw new MemoError("auction_id", "invalid auction id");
    if (!isNonce(nonce)) throw new MemoError("nonce", "invalid nonce");
    return { kind: "settlement", settlement: { auctionId, nonce } };
  }
  throw new MemoError("not_bb1", "memo does not start with BB1");
}

export function isCommitment(s: string): boolean {
  return /^[0-9a-f]{64}$/.test(s);
}

export function isSalt(s: string): boolean {
  return /^[0-9a-f]{32}$/.test(s);
}

export async function commitment(auctionId: string, bidZats: bigint, salt: string): Promise<string> {
  return sha256Hex(`${auctionId}|${bidZats.toString()}|${salt}`);
}

export function encodeCommit(c: Commit): string {
  return `BB1C|${c.auctionId}|${c.commitment}|${c.refundAddress}|${c.nonce}`;
}

export function encodeBid(bid: Bid): string {
  return `BB1|${bid.auctionId}|${bid.bidZats.toString()}|${bid.refundAddress}|${bid.nonce}`;
}

export function encodeSettlement(s: Settlement): string {
  return `BB1S|${s.auctionId}|${s.nonce}`;
}

async function sha256Hex(s: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s));
  return [...new Uint8Array(digest)].map((x) => x.toString(16).padStart(2, "0")).join("");
}

/** The bidder secret: 16 random bytes, hex. Stays in the bidder's browser; authenticates them to the service. */
export function randomSecret(): string {
  const b = new Uint8Array(16);
  crypto.getRandomValues(b);
  return [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
}

export function isSecret(s: string): boolean {
  return /^[0-9a-f]{32}$/.test(s);
}

/** The memo nonce is sha256(secret)[..16]. It travels in the memo; the secret never does. */
export async function nonceFromSecret(secret: string): Promise<string> {
  return (await sha256Hex(secret)).slice(0, 16);
}

/** @deprecated nonces are derived from a secret now; kept for tools that only need a random nonce. */
export function randomNonce(): string {
  const b = new Uint8Array(8);
  crypto.getRandomValues(b);
  return [...b].map((x) => x.toString(16).padStart(2, "0")).join("");
}

export async function receipt(nonce: string): Promise<string> {
  return sha256Hex(nonce);
}

/** sha256 over sorted `receipt:bidZats` lines; must match the service's ledger_hash. */
export async function ledgerHash(entries: Array<[string, bigint]>): Promise<string> {
  const lines = entries.map(([r, v]) => `${r}:${v.toString()}`).sort();
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(lines.join("\n")));
  return [...new Uint8Array(digest)].map((x) => x.toString(16).padStart(2, "0")).join("");
}
