export const API_BASE = process.env.NEXT_PUBLIC_BB_API ?? "http://127.0.0.1:8787";

export interface Auction {
  id: string;
  title: string;
  description: string;
  reserve_zats: number;
  deposit_zats: number;
  address: string;
  created_at: number;
  close_at: number;
  status: "open" | "closed" | "no_sale" | "settled" | "paid" | "unsettled";
  closed_at: number | null;
  ledger_hash: string | null;
  settlement_txid: string | null;
  payout_txid: string | null;
  image_url?: string | null;
  units: number;
  format: import("./formats").Format;
  clearing_zats: number | null;
  /** Last block counted; bids mined after it were late. Set once the chain passed the close time. */
  close_height?: number | null;
}

export interface AuctionView extends Auction {
  bid_count: number;
  seconds_remaining: number;
}

export interface Receipts {
  auction_id: string;
  receipts: string[];
}

export interface AuctionResult {
  auction_id: string;
  status: string;
  format: import("./formats").Format;
  units: number;
  units_sold: number;
  units_settled: number;
  clearing_zats: number | null;
  winning_amount_zats: number | null;
  winning_txid: string | null;
  winning_txids: string[];
  ledger_hash: string | null;
  valid_bid_count: number;
  refund_latency_secs: number | null;
  refunds_sent: number;
  refunds_pending: number;
  settlement_txid: string | null;
  payout_txid: string | null;
  refunds: { kind: string; status: string; txid: string | null }[];
  disclosure: unknown | null;
  price_source: "own_bid" | "lowest_winning" | "second_bid" | "reserve" | "lowest_quote" | null;
  /** Vickrey: the runner-up's ledger leaf and Merkle path proving the price is a committed bid. */
  price_proof: { receipt: string; bid_zats: number; nonce: string; proof: import("@blind-bazaar/sdk").InclusionProof } | null;
}

export interface ProofResponse {
  auction_id: string;
  ledger_hash: string;
  leaf_count: number;
  proof: import("@blind-bazaar/sdk").InclusionProof;
}

export interface LotMoney {
  bids_valid: number;
  bids_rejected: number;
  deposits_held_zats: number;
  winners: number;
  winners_settled: number;
  seller_paid_zats: number;
  seller_pending_zats: number;
  refunds_sent: number;
  refunds_pending: number;
}

export interface SellerLot extends Auction {
  seller_address: string;
  money: LotMoney;
  fulfilment: string | null;
}

export interface SellerLotDetail extends SellerLot {
  events: { id: number; kind: string; payload: Record<string, unknown>; created_at: number }[];
  payouts: { id: number; kind: string; status: string; value_zats: number; txid: string | null; not_before: number }[];
  /** RFQ, once closed: how the buyer pays the winning quote. */
  settlement?: { uri: string; memo: string; amount_zats: number; address: string; settle_by: number | null };
}

export interface Standing {
  receipt: string;
  status: string;
  outcome: "pending" | "won" | "settled" | "lost" | "forfeited" | "expired" | "rejected";
  format: import("./formats").Format;
  deposit_zats: number;
  price_zats: number | null;
  owed_zats: number;
  settle_by: number | null;
  settlement?: { uri: string; memo: string; amount_zats: number; address: string; settle_by: number | null };
  payouts: { id: number; kind: string; status: string; value_zats: number; txid: string | null }[];
  fulfilment?: string;
}

export interface Health {
  network: "test" | "main";
  chain_tip: number | null;
  wallet_height: number | null;
  sync_lag: number | null;
  last_sync_at: number | null;
  last_error: string | null;
  operator_address: string | null;
  balance_zats: number | null;
  low_balance?: boolean;
  dead_payouts?: number;
  /** "seed" or "frost 2-of-3 (...)"; null for a viewer process. */
  signing?: string | null;
}

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
    cache: "no-store",
  });
  if (!res.ok) {
    let message = res.statusText;
    try {
      message = ((await res.json()) as { error?: string }).error ?? message;
    } catch {
      /* body was not JSON */
    }
    throw new ApiError(res.status, message);
  }
  return (await res.json()) as T;
}

export const api = {
  health: () => request<Health>("/health"),
  auctions: () => request<Auction[]>("/auctions"),
  auction: (id: string) => request<AuctionView>(`/auctions/${id}`),
  receipts: (id: string) => request<Receipts>(`/auctions/${id}/receipts`),
  result: (id: string) => request<AuctionResult>(`/auctions/${id}/result`),
  proof: (id: string, receipt: string) => request<ProofResponse>(`/auctions/${id}/proof?receipt=${encodeURIComponent(receipt)}`),
  createAuction: (credential: string, body: Record<string, unknown>) =>
    request<Auction>("/auctions", {
      method: "POST",
      headers: credential.startsWith("bbs_") ? { "x-seller-key": credential } : { "x-admin-token": credential },
      body: JSON.stringify(body),
    }),
  sellerMe: (key: string) => request<{ seller_id: number; name: string }>("/seller/me", { headers: { "x-seller-key": key } }),
  sellerLots: (key: string) => request<SellerLot[]>("/seller/lots", { headers: { "x-seller-key": key } }),
  sellerLot: (key: string, id: string) => request<SellerLotDetail>(`/seller/lots/${id}`, { headers: { "x-seller-key": key } }),
  setFulfilment: (key: string, id: string, note: string) =>
    request<{ ok: boolean }>(`/seller/lots/${id}/fulfilment`, { method: "POST", headers: { "x-seller-key": key }, body: JSON.stringify({ note }) }),
  fulfilment: (id: string, secret: string) =>
    request<{ note: string | null; updated_at: number | null }>(`/auctions/${id}/fulfilment?secret=${encodeURIComponent(secret)}`),
  standing: (id: string, secret: string) => request<Standing>(`/auctions/${id}/standing?secret=${encodeURIComponent(secret)}`),
  postEnvelope: (id: string, body: Record<string, unknown>) =>
    request<{ ok: boolean; receipt: string }>(`/auctions/${id}/envelopes`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
};

export function explorerTxUrl(network: "test" | "main", txid: string): string {
  return network === "main"
    ? `https://mainnet.zcashexplorer.app/transactions/${txid}`
    : `https://testnet.zcashexplorer.app/transactions/${txid}`;
}
