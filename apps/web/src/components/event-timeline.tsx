"use client";

import { useQuery } from "@tanstack/react-query";
import { API_BASE } from "@/lib/api";
import { dateTime, zec } from "@/lib/format";
import { Eyebrow } from "@/components/ui";

interface Event {
  id: number;
  kind: string;
  payload: Record<string, unknown>;
  created_at: number;
}

const LABELS: Record<string, (p: Record<string, unknown>) => string> = {
  created: () => "Lot listed",
  bid_received: () => "Sealed bid received",
  closed: (p) => `Closed. ${p.format === "rfq" ? "Winning quote" : p.format === "vickrey" ? "Price (second bid)" : "Winning bid"} ${zec(p.winning_amount_zats as number)}`,
  no_sale: () => "Closed with no bid at reserve",
  refund_sent: () => "Refund sent, shielded",
  second_chance: (p) => `Winner forfeited. Offered to the next bid at ${zec(p.winning_amount_zats as number)}`,
  forfeited: () => "Winner did not pay in time. Deposit forfeited to seller",
  forfeit_paid: () => "Forfeited deposit paid to seller",
  unsettled: () => "No eligible bidder left. Ended unsettled",
  settled: () => "Winner paid the balance",
  seller_paid: () => "Seller paid, shielded",
};

export function EventTimeline({ auctionId }: { auctionId: string }) {
  const { data } = useQuery({
    queryKey: ["events", auctionId],
    queryFn: async () => {
      const res = await fetch(`${API_BASE}/auctions/${auctionId}/events`, { cache: "no-store" });
      if (!res.ok) throw new Error("events unavailable");
      return (await res.json()) as Event[];
    },
    refetchInterval: 10_000,
  });
  if (!data || data.length === 0) return null;
  return (
    <section aria-labelledby="timeline-heading" className="flex flex-col gap-4">
      <Eyebrow id="timeline-heading">Timeline</Eyebrow>
      <ol className="flex flex-col">
        {data.map((e, i) => (
          <li key={e.id} className={`grid grid-cols-[150px_1fr] gap-4 py-3 text-[14px] ${i > 0 ? "hairline-t" : ""}`}>
            <span className="num text-[12px] text-muted-foreground">{dateTime(e.created_at)}</span>
            <span>{(LABELS[e.kind] ?? (() => e.kind.replace(/_/g, " ")))(e.payload)}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}
