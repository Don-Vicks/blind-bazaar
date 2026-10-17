import { json, preflight } from "@/lib/cors";
import { allow, clientKey, originAllowed } from "@/lib/rate-limit";
import { submitDeposit, type Rail } from "@/lib/rails";

export async function POST(req: Request) {
  if (!originAllowed(req)) return json(req, { error: "origin not allowed" }, { status: 403 });
  if (!allow(clientKey(req))) return json(req, { error: "too many requests" }, { status: 429 });
  const body = (await req.json().catch(() => null)) as { rail?: Rail; depositAddress?: string; txHash?: string } | null;
  if (!body?.rail || !body.depositAddress || !body.txHash) {
    return json(req, { error: "rail, depositAddress and txHash required" }, { status: 400 });
  }
  try {
    await submitDeposit(body.rail, body.depositAddress, body.txHash);
    return json(req, { ok: true });
  } catch (e) {
    return json(req, { error: (e as Error).message }, { status: 502 });
  }
}

export function OPTIONS(req: Request) {
  return preflight(req);
}
