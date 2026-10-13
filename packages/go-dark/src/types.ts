export type Chain = "solana" | "ethereum";
export type Asset = "SOL" | "USDC" | "ETH";

export interface Quote {
  rail: "oneclick" | "maya";
  chain: Chain;
  asset: Asset;
  depositAddress: string;
  amountIn: string;
  amountInFormatted: string;
  amountOutZats: string;
  expiresAt: number;
  etaSeconds: number;
  trackingId: string;
  maya?: { memo: string; router: string; inboundAddress: string; tokenContract?: string };
}

export type SwapStatus = "pending" | "processing" | "success" | "refunded" | "failed";

export interface StatusResponse {
  rail: "oneclick" | "maya";
  status: SwapStatus;
  destinationTxids: string[];
  detail?: string;
}

export interface GoDarkOptions {
  /** Zcash unified address that receives the shielded ZEC. */
  to: string;
  /** Exact ZEC to deliver, in zatoshis (string or bigint). */
  amountZats: string | bigint;
  /** Origin that hosts the /api/godark/* proxy routes. Defaults to the page origin. */
  apiBase?: string;
  /** Optional sealed-bid envelope: posted to the auction service once the swap is funded. */
  envelope?: { auctionApi: string; auctionId: string; payload: string };
  /** Restrict the chains offered. Defaults to both. */
  chains?: Chain[];
  /** Label on the opening button. */
  label?: string;
  onStatus?: (event: GoDarkEvent) => void;
}

export type GoDarkEvent =
  | { type: "quoted"; quote: Quote }
  | { type: "sent"; chain: Chain; txHash: string; quote: Quote }
  | { type: "status"; status: StatusResponse }
  | { type: "error"; message: string };
