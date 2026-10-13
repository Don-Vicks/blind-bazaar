import { NextResponse } from "next/server";
import { allow, clientKey, originAllowed } from "@/lib/rate-limit";
import { getStatus, type Rail } from "@/lib/rails";

export async function GET(req: Request) {
  if (!originAllowed(req)) return NextResponse.json({ error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return NextResponse.json({ error: "too many requests" }, { status: 429 });
  const url = new URL(req.url);
  const rail = url.searchParams.get("rail") as Rail | null;
  const id = url.searchParams.get("id");
  if (!rail || !id) return NextResponse.json({ error: "rail and id required" }, { status: 400 });
  try {
    return NextResponse.json(await getStatus(rail, id), { headers: { "cache-control": "no-store" } });
  } catch (e) {
    return NextResponse.json({ error: (e as Error).message }, { status: 502 });
  }
}
