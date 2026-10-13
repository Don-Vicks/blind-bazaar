import { NextResponse } from "next/server";
import { allow, clientKey, originAllowed } from "@/lib/rate-limit";

const API = process.env.NEXT_PUBLIC_BB_API ?? "http://127.0.0.1:8787";

/** A fresh receive address from the auction service, one per payment. */
export async function GET(req: Request) {
  if (!originAllowed(req)) return NextResponse.json({ error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return NextResponse.json({ error: "too many requests" }, { status: 429 });
  try {
    const res = await fetch(`${API}/godark/receive-address`, { method: "POST", cache: "no-store" });
    if (!res.ok) return NextResponse.json({ error: "service unavailable" }, { status: 502 });
    return NextResponse.json(await res.json(), { headers: { "cache-control": "no-store" } });
  } catch {
    return NextResponse.json({ error: "service unavailable" }, { status: 502 });
  }
}
