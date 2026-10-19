/** In-memory token bucket per key (IP). Enough for a single instance; swap for Redis behind a load balancer. */
const buckets = new Map<string, { tokens: number; updated: number }>();
const CAPACITY = 20;
const REFILL_PER_SEC = 0.5;

export function allow(key: string): boolean {
  const now = Date.now();
  const b = buckets.get(key) ?? { tokens: CAPACITY, updated: now };
  b.tokens = Math.min(CAPACITY, b.tokens + ((now - b.updated) / 1000) * REFILL_PER_SEC);
  b.updated = now;
  if (b.tokens < 1) {
    buckets.set(key, b);
    return false;
  }
  b.tokens -= 1;
  buckets.set(key, b);
  // Evict the oldest entries rather than clearing: a flood of fresh keys must not reset everyone else's budget.
  if (buckets.size > 10_000) {
    for (const k of buckets.keys()) {
      buckets.delete(k);
      if (buckets.size <= 8_000) break;
    }
  }
  return true;
}

/** The caller's address for rate limiting. `x-forwarded-for` is only honoured when GODARK_TRUST_PROXY=true,
 *  because any client can send that header; otherwise `x-real-ip` (set by the reverse proxy in deploy/Caddyfile)
 *  or, without a proxy, one shared bucket. */
export function clientKey(req: Request): string {
  if (process.env.GODARK_TRUST_PROXY === "true") {
    const fwd = req.headers.get("x-forwarded-for");
    if (fwd) return fwd.split(",")[0].trim();
  }
  return req.headers.get("x-real-ip") ?? "local";
}

/** Validation for ids that go into upstream URLs. */
export const isMayaTxid = (s: string) => /^(0x)?[0-9a-fA-F]{64}$/.test(s);
export const isOneclickDeposit = (s: string) => /^[0-9a-zA-Z._:-]{8,128}$/.test(s);
export const isRail = (s: unknown): s is "maya" | "oneclick" => s === "maya" || s === "oneclick";
export const isTransparentAddress = (s: string) => /^t[13m][0-9a-zA-Z]{20,60}$/.test(s);
export const isSolanaAddress = (s: string) => /^[1-9A-HJ-NP-Za-km-z]{32,44}$/.test(s);
export const isEvmAddress = (s: string) => /^0x[0-9a-fA-F]{40}$/.test(s);

/** Upstream error text is logged, not relayed: it can carry our own addresses and the rail's internals. */
export function upstreamError(where: string, e: unknown): string {
  console.error(`[godark] ${where}: ${(e as Error).message ?? e}`);
  return `${where} is unavailable right now`;
}

/** Origins allowed to call the proxy. Empty list means any origin (development). Same-origin requests
 *  (browsers send no Origin header on same-origin GETs) are always allowed. */
export function originAllowed(req: Request): boolean {
  const allowed = (process.env.GODARK_ALLOWED_ORIGINS ?? "").split(",").map((s) => s.trim()).filter(Boolean);
  if (allowed.length === 0) return true;
  const origin = req.headers.get("origin");
  if (origin === null) {
    const site = req.headers.get("sec-fetch-site");
    return site === null || site === "same-origin" || site === "none";
  }
  return allowed.includes(origin) || origin === new URL(req.url).origin;
}
