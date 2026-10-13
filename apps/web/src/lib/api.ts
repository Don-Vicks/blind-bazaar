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
  status: "open" | "closed" | "no_sale" | "settled" | "paid";
  closed_at: number | null;
  ledger_hash: string | null;
  settlement_txid: string | null;
  payout_txid: string | null;
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
  winning_amount_zats: number | null;
  winning_txid: string | null;
  ledger_hash: string | null;
  valid_bid_count: number;
  settlement_txid: string | null;
  payout_txid: string | null;
  refunds: { kind: string; status: string; txid: string | null }[];
  disclosure: unknown | null;
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
  createAuction: (token: string, body: Record<string, unknown>) =>
    request<Auction>("/auctions", {
      method: "POST",
      headers: { "x-admin-token": token },
      body: JSON.stringify(body),
    }),
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
