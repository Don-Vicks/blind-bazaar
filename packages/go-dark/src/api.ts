import type { Chain, Asset, Quote, StatusResponse } from "./types";

export async function fetchQuote(
  apiBase: string,
  req: { chain: Chain; asset: Asset; zecOutZats: string; recipient: string; recipientTransparent?: string; refundTo: string },
): Promise<Quote> {
  const res = await fetch(`${apiBase}/api/godark/quote`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(req),
  });
  const j = (await res.json()) as Quote & { error?: string };
  if (!res.ok) throw new Error(j.error ?? `quote failed (${res.status})`);
  return j;
}

export async function submitDeposit(apiBase: string, rail: Quote["rail"], depositAddress: string, txHash: string) {
  await fetch(`${apiBase}/api/godark/submit`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ rail, depositAddress, txHash }),
  }).catch(() => undefined);
}

export async function fetchStatus(apiBase: string, rail: Quote["rail"], id: string): Promise<StatusResponse> {
  const res = await fetch(`${apiBase}/api/godark/status?rail=${rail}&id=${encodeURIComponent(id)}`);
  const j = (await res.json()) as StatusResponse & { error?: string };
  if (!res.ok) throw new Error(j.error ?? `status failed (${res.status})`);
  return j;
}

export async function postEnvelope(
  auctionApi: string,
  auctionId: string,
  body: { rail: string; payload: string },
): Promise<{ address: string; transparent?: string | null; receipt: string; id: string }> {
  const res = await fetch(`${auctionApi}/auctions/${auctionId}/envelopes`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  const j = (await res.json()) as { address: string; transparent?: string | null; receipt: string; id: string; error?: string };
  if (!res.ok) throw new Error(j.error ?? `envelope failed (${res.status})`);
  return j;
}

export async function fetchFreshAddress(endpoint: string): Promise<{ ua: string; transparent?: string }> {
  const res = await fetch(endpoint, { cache: "no-store" });
  const j = (await res.json()) as { ua: string; transparent?: string; error?: string };
  if (!res.ok || !j.ua) throw new Error(j.error ?? "could not get a receive address");
  return j;
}
