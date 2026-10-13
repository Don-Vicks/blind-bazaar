export type Rail = "oneclick" | "maya";
export type Chain = "solana" | "ethereum";
export type Asset = "SOL" | "USDC" | "ETH";

export interface QuoteRequest {
  chain: Chain;
  asset: Asset;
  /** Exact ZEC the recipient must receive, in zatoshis. */
  zecOutZats: string;
  /** Zcash unified (u1…) or transparent (t1…) address. */
  recipient: string;
  /** Transparent receiver of the same unified address, used only if the rail rejects the unified one. */
  recipientTransparent?: string;
  /** Payer's address on the source chain, for refunds. */
  refundTo: string;
  dry?: boolean;
}

export interface Quote {
  rail: Rail;
  chain: Chain;
  asset: Asset;
  /** Where the payer sends funds on the source chain. */
  depositAddress: string;
  /** Amount to send, in the source asset's base units (lamports, USDC 6dp, wei). */
  amountIn: string;
  amountInFormatted: string;
  /** Expected ZEC delivered, zatoshis. */
  amountOutZats: string;
  /** Unix seconds after which the quote must not be used. */
  expiresAt: number;
  /** Seconds the rail estimates for delivery. */
  etaSeconds: number;
  /** Identifier to poll status with: 1Click deposit address or Maya inbound txid (filled after send). */
  trackingId: string;
  /** `shielded`: the rail delivers straight to the unified address. `transparent`: it delivers to the
   *  transparent receiver and the service shields on arrival. */
  delivery: "shielded" | "transparent";
  /** Rail-specific extras the widget needs to build the source-chain transaction. */
  maya?: { memo: string; router: string; inboundAddress: string; tokenContract?: string };
}

export type SwapStatus = "pending" | "processing" | "success" | "refunded" | "failed";

export interface StatusResponse {
  rail: Rail;
  status: SwapStatus;
  destinationTxids: string[];
  detail?: string;
}
