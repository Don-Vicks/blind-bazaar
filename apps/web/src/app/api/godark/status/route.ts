import { json, preflight } from "@/lib/cors";
import { allow, clientKey, isMayaTxid, isOneclickDeposit, isRail, originAllowed, upstreamError } from "@/lib/rate-limit";
import { getStatus, type Rail } from "@/lib/rails";

export async function GET(req: Request) {
  if (!originAllowed(req)) return json(req, { error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return json(req, { error: "too many requests" }, { status: 429 });
  const url = new URL(req.url);
  const rail = url.searchParams.get("rail");
  const id = url.searchParams.get("id") ?? "";
  if (!isRail(rail) || !id) return json(req, { error: "rail and id required" }, { status: 400 });
  if (rail === "maya" ? !isMayaTxid(id) : !isOneclickDeposit(id)) return json(req, { error: "bad id" }, { status: 400 });
  try {
    return json(req, await getStatus(rail as Rail, id), { headers: { "cache-control": "no-store" } });
  } catch (e) {
    return json(req, { error: upstreamError("status", e) }, { status: 502 });
  }
}

export function OPTIONS(req: Request) {
  return preflight(req);
}
