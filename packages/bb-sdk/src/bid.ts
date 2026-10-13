import { type Bid, encodeBid, randomNonce, receipt } from "./memo";
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
}

/** Everything a bidder needs: the memo, the ZIP-321 URI for their wallet, and the receipt to keep. */
export async function prepareBid(auction: AuctionLike, bidZats: bigint, refundAddress: string): Promise<PreparedBid> {
  const depositZats = BigInt(auction.deposit_zats);
  const bid: Bid = { auctionId: auction.id, bidZats, refundAddress, nonce: randomNonce() };
  const memo = encodeBid(bid);
  return {
    bid,
    memo,
    uri: buildPaymentUri({ address: auction.address, amountZats: depositZats, memo }),
    receipt: await receipt(bid.nonce),
    depositZats,
  };
}
