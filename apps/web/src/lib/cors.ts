import { NextResponse } from "next/server";

/**
 * CORS for the Go Dark proxy routes, so a pay button on any site can call them. The allow-list in
 * GODARK_ALLOWED_ORIGINS decides who; an empty list (development) reflects any origin.
 */
export function corsHeaders(req: Request): Record<string, string> {
  const origin = req.headers.get("origin");
  if (!origin) return {};
  const allowed = (process.env.GODARK_ALLOWED_ORIGINS ?? "").split(",").map((s) => s.trim()).filter(Boolean);
  if (allowed.length > 0 && !allowed.includes(origin)) return {};
  return {
    "access-control-allow-origin": origin,
    "access-control-allow-methods": "GET, POST, OPTIONS",
    "access-control-allow-headers": "content-type",
    "access-control-max-age": "600",
    vary: "origin",
  };
}

/** Preflight response for the proxy routes. */
export function preflight(req: Request) {
  return new NextResponse(null, { status: 204, headers: corsHeaders(req) });
}

/** A JSON response carrying the CORS headers for `req`. */
export function json(req: Request, body: unknown, init?: { status?: number; headers?: Record<string, string> }) {
  return NextResponse.json(body, { status: init?.status ?? 200, headers: { ...corsHeaders(req), ...(init?.headers ?? {}) } });
}
