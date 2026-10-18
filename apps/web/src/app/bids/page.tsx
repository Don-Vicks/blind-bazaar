"use client";

import { useQueries, useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { allSavedBids, downloadCard, toCard, type SavedBidRecord } from "@/lib/bid-card";
import { zec } from "@/lib/format";
import { Button, Eyebrow, LinkButton, Panel, StatusDot } from "@/components/ui";

const LABEL: Record<string, string> = {
  pending: "In, waiting for close",
  sealed: "Sealed, waiting for close",
  reveal_now: "Reveal now",
  revealed: "Revealed, waiting for the window to end",
  won: "Won, pay the balance",
  settled: "Won and paid",
  lost: "Lost, refund on its way",
  forfeited: "Forfeited",
  expired: "Buyer did not pay",
  unrevealed: "Not revealed in time, refunded",
  rejected: "Not counted",
};

/** Every bid this browser holds a key for, across lots, with its standing. */
export default function MyBidsPage() {
  const [rows, setRows] = useState<{ auctionId: string; rec: SavedBidRecord }[]>([]);
  useEffect(() => {
    const all = allSavedBids();
    setRows(Object.entries(all).flatMap(([auctionId, list]) => list.filter((r) => r.secret).map((rec) => ({ auctionId, rec }))));
  }, []);
  const lots = useQuery({ queryKey: ["auctions"], queryFn: api.auctions });
  const standings = useQueries({
    queries: rows.map(({ auctionId, rec }) => ({
      queryKey: ["standing", auctionId, rec.secret],
      queryFn: () => api.standing(auctionId, rec.secret!),
      retry: false,
      refetchInterval: 30_000,
    })),
  });
  const attention = rows.filter((_, i) => ["reveal_now", "won"].includes(standings[i]?.data?.outcome ?? "")).length;

  return (
    <div className="flex flex-col gap-10">
      <div className="rise">
        <Eyebrow>My bids</Eyebrow>
        <h1 className="serif mt-4 text-[44px] leading-[1.02] sm:text-[64px]">{rows.length === 0 ? "No bids in this browser." : attention > 0 ? `${attention} need${attention === 1 ? "s" : ""} you.` : "All quiet."}</h1>
        <p className="mt-6 max-w-[52ch] text-[16px] leading-relaxed text-muted-foreground">
          Bids are listed from the keys saved in this browser. Nothing here is sent anywhere except each bid&apos;s own key to the auction, to fetch its standing. To see a bid made on another device, open its bid card or link on that lot&apos;s status page.
        </p>
      </div>
      {rows.length === 0 ? (
        <LinkButton href="/" variant="secondary" className="self-start">See open lots</LinkButton>
      ) : (
        <ul className="flex flex-col gap-4">
          {rows.map(({ auctionId, rec }, i) => {
            const s = standings[i];
            const lot = lots.data?.find((a) => a.id === auctionId);
            const outcome = s?.data?.outcome;
            const urgent = outcome === "reveal_now" || outcome === "won";
            return (
              <li key={rec.secret}>
                <Panel className={`flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between ${urgent ? "border-accent" : ""}`}>
                  <div className="min-w-0">
                    <div className="flex items-center gap-3">
                      <Link href={`/a/${auctionId}`} className="serif text-[22px] leading-tight hover:text-accent">{lot?.title ?? `Lot ${auctionId}`}</Link>
                      {lot && <StatusDot status={lot.status} />}
                    </div>
                    <p className="mt-1 text-[14px] text-muted-foreground">
                      {rec.bid ? `${zec(Number(rec.bid))} · ` : ""}
                      {s?.isError ? "Not seen by the auction yet" : s?.data ? (LABEL[outcome ?? ""] ?? outcome) : "Checking…"}
                      {s?.data?.owed_zats ? ` · ${zec(s.data.owed_zats)} owed` : ""}
                    </p>
                  </div>
                  <div className="flex shrink-0 gap-2">
                    <Button variant="secondary" className="min-h-9 px-3 text-[12px]" onClick={() => downloadCard(toCard(auctionId, { ...rec, secret: rec.secret! }))}>Save card</Button>
                    <LinkButton href={`/a/${auctionId}/status`} className={urgent ? "" : "min-h-9 px-3 text-[12px]"} variant={urgent ? "primary" : "secondary"}>
                      {outcome === "reveal_now" ? "Reveal" : outcome === "won" ? "Pay" : "Open"}
                    </LinkButton>
                  </div>
                </Panel>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
