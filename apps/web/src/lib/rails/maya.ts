import type { Quote, QuoteRequest, StatusResponse } from "./types";

const NODE = "https://mayanode.mayachain.info";
const USDC_ETH = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

function fromAsset(req: QuoteRequest): string {
  if (req.chain !== "ethereum") throw new Error("Maya has no Solana support");
  return req.asset === "ETH" ? "ETH.ETH" : `ETH.USDC-${USDC_ETH}`;
}

interface MayaQuote {
  inbound_address: string;
  router?: string;
  memo: string;
  expiry: number;
  expected_amount_out: string;
  recommended_min_amount_in: string;
  total_swap_seconds: number;
  fees: { total_bps: number; outbound: string };
}

async function quoteExactIn(asset: string, amountIn1e8: bigint, destination: string): Promise<MayaQuote> {
  const url = `${NODE}/mayachain/quote/swap?from_asset=${encodeURIComponent(asset)}&to_asset=ZEC.ZEC&amount=${amountIn1e8}&destination=${destination}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Maya quote failed (${res.status}): ${await res.text()}`);
  return (await res.json()) as MayaQuote;
}

/** Maya quotes are exact-input, so we solve for the input that yields the wanted ZEC with a 1.5% buffer. */
export async function mayaQuote(req: QuoteRequest): Promise<Quote> {
  const asset = fromAsset(req);
  const want = BigInt(req.zecOutZats);
  // Probe with a round number, then scale.
  const probeIn = asset === "ETH.ETH" ? 1_000_000n /* 0.01 ETH in 1e8 units */ : 10_000_000_000n; /* 100 USDC */
  const probe = await quoteExactIn(asset, probeIn, req.recipient);
  const probeOut = BigInt(probe.expected_amount_out);
  if (probeOut === 0n) throw new Error("Maya returned zero output");
  let amountIn = (probeIn * want * 1015n) / (probeOut * 1000n);
  const minIn = BigInt(probe.recommended_min_amount_in);
  if (amountIn < minIn) amountIn = minIn;
  const final = await quoteExactIn(asset, amountIn, req.recipient);
  if (BigInt(final.expected_amount_out) < want) {
    amountIn = (amountIn * 1030n) / 1000n;
  }
  // Convert 1e8 units to native base units: ETH 18dp, USDC 6dp.
  const native = asset === "ETH.ETH" ? amountIn * 10_000_000_000n : amountIn / 100n;
  const formatted = asset === "ETH.ETH" ? `${Number(amountIn) / 1e8} ETH` : `${Number(amountIn) / 1e8} USDC`;
  return {
    rail: "maya",
    chain: "ethereum",
    asset: req.asset,
    depositAddress: final.inbound_address,
    amountIn: native.toString(),
    amountInFormatted: formatted,
    amountOutZats: final.expected_amount_out,
    expiresAt: final.expiry,
    etaSeconds: final.total_swap_seconds,
    trackingId: "",
    maya: {
      memo: final.memo,
      router: final.router ?? "",
      inboundAddress: final.inbound_address,
      tokenContract: asset === "ETH.ETH" ? undefined : USDC_ETH,
    },
  };
}

export async function mayaStatus(inboundTxid: string): Promise<StatusResponse> {
  const res = await fetch(`${NODE}/mayachain/tx/status/${inboundTxid.replace(/^0x/, "")}`);
  if (res.status === 404) return { rail: "maya", status: "pending", destinationTxids: [], detail: "not observed yet" };
  if (!res.ok) throw new Error(`Maya status failed (${res.status})`);
  const j = (await res.json()) as {
    stages?: { swap_finalised?: { completed?: boolean }; outbound_signed?: { completed?: boolean } };
    out_txs?: { id?: string }[];
  };
  const outs = (j.out_txs ?? []).map((t) => t.id ?? "").filter(Boolean);
  const done = j.stages?.outbound_signed?.completed || outs.length > 0;
  return {
    rail: "maya",
    status: done ? "success" : "processing",
    destinationTxids: outs,
  };
}
