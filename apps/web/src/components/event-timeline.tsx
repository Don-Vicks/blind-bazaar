"use client";

import { useQuery } from "@tanstack/react-query";
import { API_BASE } from "@/lib/api";
import { dateTime, zec } from "@/lib/format";

interface Event {
  id: number;
  kind: string;
  payload: Record<string, unknown>;
  created_at: number;
}

const LABELS: Record<string, (p: Record<string, unknown>) => string> = {
  created: () => "Auction listed",
  bid_received: () => "Sealed bid received",
  closed: (p) => `Closed. Winning bid ${zec(p.winning_amount_zats as number)}`,
  no_sale: () => "Closed with no bid at reserve",
  refund_sent: () => "Refund sent, shielded",
  second_chance: (p) => `Winner forfeited. Offered to the next bid at ${zec(p.winning_amount_zats as number)}`,
  forfeited: () => "Winner did not pay in time. Deposit forfeited to seller",
  forfeit_paid: () => "Forfeited deposit paid to seller",
  unsettled: () => "No eligible bidder left. Auction ended unsettled",
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
    <section aria-labelledby="timeline-heading" className="flex flex-col gap-3">
      <h2 id="timeline-heading" className="text-sm font-medium">
        Timeline
      </h2>
      <ol className="flex flex-col gap-2 border-l border-border pl-4 text-sm">
        {data.map((e) => (
          <li key={e.id} className="relative">
            <span aria-hidden className="absolute -left-[21px] top-1.5 size-2 rounded-full bg-accent" />
            <p>{(LABELS[e.kind] ?? (() => e.kind.replace(/_/g, " ")))(e.payload)}</p>
            <p className="text-xs text-muted-foreground">{dateTime(e.created_at)}</p>
          </li>
        ))}
      </ol>
    </section>
  );
}
