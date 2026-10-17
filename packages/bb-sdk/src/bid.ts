import { type Bid, encodeBid, nonceFromSecret, randomSecret, receipt } from "./memo";
import { buildPaymentUri } from "./zip321";

export interface AuctionLike {
  id: string;
  address: string;
  deposit_zats: number | bigint | string;
}

export interface PreparedBid {
  bid: Bid;
  memo: string;
  uri: string;
  receipt: string;
  depositZats: bigint;
  /** Keep this. It proves the bid is yours to the service (standing, delivery note). Never in the memo. */
  secret: string;
}

/** Everything a bidder needs: the memo, the ZIP-321 URI for their wallet, and the receipt to keep. */
export async function prepareBid(auction: AuctionLike, bidZats: bigint, refundAddress: string): Promise<PreparedBid> {
  const depositZats = BigInt(auction.deposit_zats);
  const secret = randomSecret();
  const bid: Bid = { auctionId: auction.id, bidZats, refundAddress, nonce: await nonceFromSecret(secret) };
  const memo = encodeBid(bid);
  return {
    bid,
    memo,
    uri: buildPaymentUri({ address: auction.address, amountZats: depositZats, memo }),
    receipt: await receipt(bid.nonce),
    depositZats,
    secret,
  };
}
