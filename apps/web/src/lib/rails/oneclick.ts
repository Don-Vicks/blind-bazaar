import type { Quote, QuoteRequest, StatusResponse } from "./types";

const BASE = "https://1click.chaindefuser.com";

export const ONECLICK_ASSETS = {
  ZEC: "nep141:zec.omft.near",
  SOL: "nep141:sol.omft.near",
  USDC_SOL: "nep141:sol-5ce3bf3a31af18be40ba30f721101b4341690186.omft.near",
  ETH: "nep141:eth.omft.near",
  USDC_ETH: "nep141:eth-0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.omft.near",
} as const;

function originAsset(req: QuoteRequest): string {
  if (req.chain === "solana") return req.asset === "SOL" ? ONECLICK_ASSETS.SOL : ONECLICK_ASSETS.USDC_SOL;
  return req.asset === "ETH" ? ONECLICK_ASSETS.ETH : ONECLICK_ASSETS.USDC_ETH;
}

function headers(): Record<string, string> {
  const h: Record<string, string> = { "content-type": "application/json" };
  const jwt = process.env.ONECLICK_JWT;
  if (jwt) h["x-api-key"] = jwt;
  return h;
}

export async function oneclickQuote(req: QuoteRequest): Promise<Quote> {
  try {
    return await oneclickQuoteTo(req, req.recipient, "shielded");
  } catch (e) {
    // Official docs still list Zcash as transparent-only; fall back to the transparent receiver if offered.
    if (req.recipientTransparent && /recipient|address|unsupported|invalid/i.test((e as Error).message)) {
      return oneclickQuoteTo(req, req.recipientTransparent, "transparent");
    }
    throw e;
  }
}

async function oneclickQuoteTo(req: QuoteRequest, recipient: string, delivery: Quote["delivery"]): Promise<Quote> {
  const deadline = new Date(Date.now() + 20 * 60_000).toISOString();
  const body = {
    dry: req.dry ?? false,
    swapType: "EXACT_OUTPUT",
    slippageTolerance: 100,
    originAsset: originAsset(req),
    depositType: "ORIGIN_CHAIN",
    destinationAsset: ONECLICK_ASSETS.ZEC,
    amount: req.zecOutZats,
    refundTo: req.refundTo,
    refundType: "ORIGIN_CHAIN",
    recipient,
    recipientType: "DESTINATION_CHAIN",
    deadline,
    depositMode: "SIMPLE",
    referral: "godark",
  };
  const res = await fetch(`${BASE}/v0/quote`, { method: "POST", headers: headers(), body: JSON.stringify(body) });
  if (!res.ok) throw new Error(`1Click quote failed (${res.status}): ${await res.text()}`);
  const j = (await res.json()) as {
    quote: {
      depositAddress?: string;
      amountIn: string;
      amountInFormatted: string;
      amountOut: string;
      timeEstimate: number;
      deadline?: string;
    };
  };
  return {
    rail: "oneclick",
    chain: req.chain,
    asset: req.asset,
    depositAddress: j.quote.depositAddress ?? "",
    amountIn: j.quote.amountIn,
    amountInFormatted: j.quote.amountInFormatted,
    amountOutZats: j.quote.amountOut,
    expiresAt: Math.floor(new Date(j.quote.deadline ?? deadline).getTime() / 1000),
    etaSeconds: j.quote.timeEstimate,
    trackingId: j.quote.depositAddress ?? "",
    delivery,
  };
}

export async function oneclickSubmit(depositAddress: string, txHash: string): Promise<void> {
  const res = await fetch(`${BASE}/v0/deposit/submit`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({ depositAddress, txHash }),
  });
  if (!res.ok) throw new Error(`1Click submit failed (${res.status}): ${await res.text()}`);
}

export async function oneclickStatus(depositAddress: string): Promise<StatusResponse> {
  const res = await fetch(`${BASE}/v0/status?depositAddress=${encodeURIComponent(depositAddress)}`, {
    headers: headers(),
  });
  if (!res.ok) throw new Error(`1Click status failed (${res.status})`);
  const j = (await res.json()) as {
    status: string;
    swapDetails?: { destinationChainTxHashes?: { hash: string }[]; refundReason?: string };
  };
  const map: Record<string, StatusResponse["status"]> = {
    PENDING_DEPOSIT: "pending",
    KNOWN_DEPOSIT_TX: "processing",
    INCOMPLETE_DEPOSIT: "pending",
    PROCESSING: "processing",
    SUCCESS: "success",
    REFUNDED: "refunded",
    FAILED: "failed",
  };
  return {
    rail: "oneclick",
    status: map[j.status] ?? "processing",
    destinationTxids: (j.swapDetails?.destinationChainTxHashes ?? []).map((h) => h.hash),
    detail: j.swapDetails?.refundReason ?? j.status,
  };
}
