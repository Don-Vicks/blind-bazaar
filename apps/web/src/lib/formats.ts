export type Format = "first_price" | "uniform" | "vickrey" | "rfq";

/** Copy for each auction format: labels and the one-line rule shown on lot, bid and result pages. */
export const FORMATS: Record<
  Format,
  {
    name: string;
    tag: string | null;
    floorLabel: string;
    depositLabel: string;
    amountLabel: string;
    verb: string;
    rule: string;
    settle: string;
  }
> = {
  first_price: {
    name: "Sealed first price",
    tag: null,
    floorLabel: "Reserve",
    depositLabel: "Deposit to bid",
    amountLabel: "Your bid",
    verb: "bid",
    rule: "Highest bid at or above reserve wins and pays its own bid. A hash of the bid ledger is published.",
    settle: "Losers are refunded shielded. The winner pays the balance. Only the winning output is proven.",
  },
  uniform: {
    name: "Edition, uniform price",
    tag: "Edition",
    floorLabel: "Reserve",
    depositLabel: "Deposit to bid",
    amountLabel: "Your bid",
    verb: "bid",
    rule: "The top N bids at or above reserve win and every winner pays the lowest winning bid. A hash of the bid ledger is published.",
    settle: "Losers are refunded shielded. Each winner pays the balance. Only the winning outputs are proven.",
  },
  vickrey: {
    name: "Vickrey, second price",
    tag: "Vickrey",
    floorLabel: "Reserve",
    depositLabel: "Deposit to bid",
    amountLabel: "Your bid",
    verb: "bid",
    rule: "Highest bid wins but pays the second-highest bid, or the reserve if there is none. Bid what the lot is worth to you; the winning bid itself is never revealed.",
    settle: "Losers are refunded shielded. The winner pays the second price. The price is proven from the ledger without opening anyone's bid.",
  },
  rfq: {
    name: "Request for quotes",
    tag: "RFQ",
    floorLabel: "Budget",
    depositLabel: "Bond to quote",
    amountLabel: "Your quote",
    verb: "quote",
    rule: "The lowest quote at or below the budget wins. Quotes stay sealed; the buyer learns only the winning price. A hash of the quote ledger is published.",
    settle: "Losing bonds are refunded shielded. The buyer pays the winning quote; the winner receives it plus their bond.",
  },
};

export function fmt(format: string | undefined) {
  return FORMATS[(format as Format) in FORMATS ? (format as Format) : "first_price"];
}

export function isRfq(format: string | undefined) {
  return format === "rfq";
}
