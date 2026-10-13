import { NextResponse } from "next/server";
import { allow, clientKey, originAllowed } from "@/lib/rate-limit";
import { getQuote, isAcceptableZcashAddress, type QuoteRequest } from "@/lib/rails";

export async function POST(req: Request) {
  if (!originAllowed(req)) return NextResponse.json({ error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return NextResponse.json({ error: "too many requests" }, { status: 429 });
  let body: QuoteRequest;
  try {
    body = (await req.json()) as QuoteRequest;
  } catch {
    return NextResponse.json({ error: "invalid JSON" }, { status: 400 });
  }
  if (!["solana", "ethereum"].includes(body.chain) || !["SOL", "USDC", "ETH"].includes(body.asset)) {
    return NextResponse.json({ error: "unsupported chain or asset" }, { status: 400 });
  }
  if (!isAcceptableZcashAddress(body.recipient)) {
    return NextResponse.json({ error: "recipient must be a u1 or t1 Zcash address" }, { status: 400 });
  }
  if (!/^\d+$/.test(body.zecOutZats) || BigInt(body.zecOutZats) < 10_000n) {
    return NextResponse.json({ error: "zecOutZats must be at least 10000" }, { status: 400 });
  }
  try {
    return NextResponse.json(await getQuote(body), { headers: { "cache-control": "no-store" } });
  } catch (e) {
    return NextResponse.json({ error: (e as Error).message }, { status: 502 });
  }
}
