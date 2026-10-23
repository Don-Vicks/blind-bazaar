import { NextResponse, type NextRequest } from "next/server";

/** The same-origin API forward (`/bb/*`, see next.config.ts) must never expose the operator's endpoints.
 *  Admin and ledger routes answer 403 here, matching the Caddy deployment. */
export function proxy(req: NextRequest) {
  const p = req.nextUrl.pathname;
  if (p.startsWith("/bb/admin") || p.startsWith("/bb/ledger")) {
    return new NextResponse(JSON.stringify({ error: "not reachable from outside" }), { status: 403, headers: { "content-type": "application/json" } });
  }
  return NextResponse.next();
}

export const config = { matcher: ["/bb/:path*"] };
