import { json, preflight } from "@/lib/cors";
import { allow, clientKey, originAllowed } from "@/lib/rate-limit";
import { getStatus, type Rail } from "@/lib/rails";

export async function GET(req: Request) {
  if (!originAllowed(req)) return json(req, { error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return json(req, { error: "too many requests" }, { status: 429 });
  const url = new URL(req.url);
  const rail = url.searchParams.get("rail") as Rail | null;
  const id = url.searchParams.get("id");
  if (!rail || !id) return json(req, { error: "rail and id required" }, { status: 400 });
  try {
    return json(req, await getStatus(rail, id), { headers: { "cache-control": "no-store" } });
  } catch (e) {
    return json(req, { error: (e as Error).message }, { status: 502 });
  }
}

export function OPTIONS(req: Request) {
  return preflight(req);
}
