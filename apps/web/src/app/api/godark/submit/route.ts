import { NextResponse } from "next/server";
import { submitDeposit, type Rail } from "@/lib/rails";

export async function POST(req: Request) {
  const body = (await req.json().catch(() => null)) as { rail?: Rail; depositAddress?: string; txHash?: string } | null;
  if (!body?.rail || !body.depositAddress || !body.txHash) {
    return NextResponse.json({ error: "rail, depositAddress and txHash required" }, { status: 400 });
  }
  try {
    await submitDeposit(body.rail, body.depositAddress, body.txHash);
    return NextResponse.json({ ok: true });
  } catch (e) {
    return NextResponse.json({ error: (e as Error).message }, { status: 502 });
  }
}
