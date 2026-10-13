/** ZIP-321 payment URI for a bid: `zcash:<ua>?amount=<zec>&memo=<base64url>`. */

export const ZATS_PER_ZEC = 100_000_000n;

/** Decimal ZEC string with up to 8 places and no trailing zeros, as ZIP-321 requires. */
export function formatZec(zats: bigint): string {
  if (zats < 0n) throw new Error("negative amount");
  const whole = zats / ZATS_PER_ZEC;
  const frac = (zats % ZATS_PER_ZEC).toString().padStart(8, "0").replace(/0+$/, "");
  return frac.length ? `${whole}.${frac}` : `${whole}`;
}

export function parseZec(s: string): bigint {
  const m = /^(\d+)(?:\.(\d{1,8}))?$/.exec(s.trim());
  if (!m) throw new Error("invalid ZEC amount");
  const whole = BigInt(m[1]);
  const frac = BigInt((m[2] ?? "").padEnd(8, "0"));
  return whole * ZATS_PER_ZEC + frac;
}

/** base64url without padding, per ZIP-321. */
export function memoToBase64Url(memo: string): string {
  const bytes = new TextEncoder().encode(memo);
  if (bytes.length > 512) throw new Error("memo exceeds 512 bytes");
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function memoFromBase64Url(s: string): string {
  if (/[+/=]/.test(s)) throw new Error("memo must be base64url without padding");
  const b64 = s.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (s.length % 4)) % 4);
  const bin = atob(b64);
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}

export interface PaymentUri {
  address: string;
  amountZats: bigint;
  memo?: string;
}

export function buildPaymentUri(p: PaymentUri): string {
  if (!p.address.startsWith("u") && p.memo) {
    throw new Error("memos are only valid for shielded (unified) addresses");
  }
  const params = [`amount=${formatZec(p.amountZats)}`];
  if (p.memo) params.push(`memo=${memoToBase64Url(p.memo)}`);
  return `zcash:${p.address}?${params.join("&")}`;
}

export function parsePaymentUri(uri: string): PaymentUri {
  const m = /^zcash:([0-9a-zA-Z]+)\?(.*)$/.exec(uri);
  if (!m) throw new Error("not a single-payment zcash: URI");
  const out: PaymentUri = { address: m[1], amountZats: 0n };
  for (const kv of m[2].split("&")) {
    const [k, v] = kv.split("=");
    if (k === "amount") out.amountZats = parseZec(v);
    else if (k === "memo") out.memo = memoFromBase64Url(v);
  }
  return out;
}
