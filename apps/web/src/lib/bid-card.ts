/** A bid card: everything a bidder needs to prove a bid is theirs, on any device. Stored in this browser
 *  under bb.receipts; exported as a file or a link so losing the browser does not lose the bid. */

export interface BidCard {
  version: 1;
  site: string;
  auction_id: string;
  receipt: string;
  nonce: string;
  secret: string;
  /** Bid amount in zatoshis, as a string. */
  bid?: string;
  /** Sealed-reveal lots: the reveal code. */
  salt?: string;
  created_at: number;
}

export interface SavedBidRecord {
  receipt: string;
  nonce: string;
  secret?: string;
  bid?: string;
  salt?: string;
}

const KEY = "bb.receipts";

export function allSavedBids(): Record<string, SavedBidRecord[]> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "{}") as Record<string, SavedBidRecord[]>;
  } catch {
    return {};
  }
}

export function saveBidRecord(auctionId: string, rec: SavedBidRecord): boolean {
  try {
    const all = allSavedBids();
    const list = (all[auctionId] ??= []);
    if (!list.some((b) => b.secret === rec.secret || b.receipt === rec.receipt)) list.push(rec);
    localStorage.setItem(KEY, JSON.stringify(all));
    return true;
  } catch {
    return false;
  }
}

export function toCard(auctionId: string, rec: SavedBidRecord & { secret: string }): BidCard {
  return {
    version: 1,
    site: typeof window !== "undefined" ? window.location.origin : "",
    auction_id: auctionId,
    receipt: rec.receipt,
    nonce: rec.nonce,
    secret: rec.secret,
    bid: rec.bid,
    salt: rec.salt,
    created_at: Math.floor(Date.now() / 1000),
  };
}

/** A link that imports the card when opened on the status page. The key travels in the fragment, which
 *  browsers never send to a server. */
export function cardLink(card: BidCard): string {
  const q = new URLSearchParams();
  q.set("key", card.secret);
  if (card.bid) q.set("bid", card.bid);
  if (card.salt) q.set("salt", card.salt);
  return `${card.site}/a/${card.auction_id}/status#${q.toString()}`;
}

/** Parse the fragment of an import link. */
export function parseCardFragment(hash: string): { secret: string; bid?: string; salt?: string } | null {
  const h = hash.startsWith("#") ? hash.slice(1) : hash;
  if (!h) return null;
  const q = new URLSearchParams(h);
  const secret = q.get("key");
  if (!secret || !/^[0-9a-f]{32}$/.test(secret)) return null;
  return { secret, bid: q.get("bid") ?? undefined, salt: q.get("salt") ?? undefined };
}

export function downloadCard(card: BidCard) {
  const blob = new Blob([JSON.stringify(card, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `blind-bazaar-bid-${card.auction_id}-${card.receipt.slice(0, 8)}.json`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function parseCardFile(text: string): BidCard | null {
  try {
    const c = JSON.parse(text) as Partial<BidCard>;
    if (c.version !== 1 || !c.auction_id || !c.secret || !/^[0-9a-f]{32}$/.test(c.secret) || !c.nonce || !c.receipt) return null;
    return c as BidCard;
  } catch {
    return null;
  }
}
