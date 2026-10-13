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
  if (buckets.size > 10_000) buckets.clear();
  return true;
}

export function clientKey(req: Request): string {
  const fwd = req.headers.get("x-forwarded-for");
  return (fwd ? fwd.split(",")[0] : req.headers.get("x-real-ip")) ?? "local";
}

/** Origins allowed to call the proxy. Empty list means any origin (development). */
export function originAllowed(req: Request): boolean {
  const allowed = (process.env.GODARK_ALLOWED_ORIGINS ?? "").split(",").map((s) => s.trim()).filter(Boolean);
  if (allowed.length === 0) return true;
  const origin = req.headers.get("origin");
  return origin !== null && allowed.includes(origin);
}
