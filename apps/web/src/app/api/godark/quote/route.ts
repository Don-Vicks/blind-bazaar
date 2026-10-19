import { json, preflight } from "@/lib/cors";
import { allow, clientKey, isEvmAddress, isSolanaAddress, isTransparentAddress, originAllowed, upstreamError } from "@/lib/rate-limit";
import { getQuote, isAcceptableZcashAddress, type QuoteRequest } from "@/lib/rails";

export async function POST(req: Request) {
  if (!originAllowed(req)) return json(req, { error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return json(req, { error: "too many requests" }, { status: 429 });
  let body: QuoteRequest;
  try {
    body = (await req.json()) as QuoteRequest;
  } catch {
    return json(req, { error: "invalid JSON" }, { status: 400 });
  }
  if (!["solana", "ethereum"].includes(body.chain) || !["SOL", "USDC", "ETH"].includes(body.asset)) {
    return json(req, { error: "unsupported chain or asset" }, { status: 400 });
  }
  if (!isAcceptableZcashAddress(body.recipient)) {
    return json(req, { error: "recipient must be a u1 or t1 Zcash address" }, { status: 400 });
  }
  if (!/^\d+$/.test(body.zecOutZats) || BigInt(body.zecOutZats) < 10_000n) {
    return json(req, { error: "zecOutZats must be at least 10000" }, { status: 400 });
  }
  if (body.recipientTransparent !== undefined && !isTransparentAddress(body.recipientTransparent)) {
    return json(req, { error: "recipientTransparent must be a transparent Zcash address" }, { status: 400 });
  }
  if (typeof body.refundTo !== "string" || !(body.chain === "solana" ? isSolanaAddress(body.refundTo) : isEvmAddress(body.refundTo))) {
    return json(req, { error: "refundTo must be an address on the source chain" }, { status: 400 });
  }
  try {
    return json(req, await getQuote(body), { headers: { "cache-control": "no-store" } });
  } catch (e) {
    return json(req, { error: upstreamError("quote", e) }, { status: 502 });
  }
}

export function OPTIONS(req: Request) {
  return preflight(req);
}
