"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { api } from "@/lib/api";
import { countdown, dateTime, zec } from "@/lib/format";
import { useNow } from "@/lib/use-now";
import { Eyebrow, Fact, LinkButton, Mono, Panel, Skeleton, StatusDot } from "@/components/ui";
import { BidPanel } from "@/components/bid-panel";
import { LotImage } from "@/components/lot-plate";
import { EventTimeline } from "@/components/event-timeline";

export function AuctionClient({ id }: { id: string }) {
  const { data, isPending, isError } = useQuery({
    queryKey: ["auction", id],
    queryFn: () => api.auction(id),
    refetchInterval: 10_000,
  });
  const now = useNow();

  if (isPending) {
    return (
      <div className="grid gap-10 sm:grid-cols-12">
        <div className="sm:col-span-7">
          <Skeleton className="h-4 w-24" />
          <Skeleton className="mt-4 h-16 w-3/4" />
          <Skeleton className="mt-8 h-24" />
        </div>
        <Skeleton className="h-80 sm:col-span-5" />
      </div>
    );
  }
  if (isError) {
    return (
      <div className="flex flex-col items-start gap-4">
        <p className="serif text-[28px]">This lot does not exist.</p>
        <LinkButton href="/" variant="secondary">
          Back to lots
        </LinkButton>
      </div>
    );
  }

  const remaining = data.close_at - (now ?? data.close_at);
  const open = data.status === "open" && remaining > 0;

  return (
    <div className="flex flex-col gap-10 sm:gap-14">
      <div className="plate rise">
        <LotImage id={data.id} src={data.image_url} alt={data.title} ratio="16/7" />
      </div>
    <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
      <div className="flex flex-col gap-10 sm:col-span-7">
        <div className="rise">
          <div className="flex items-center gap-4">
            <Eyebrow>Lot {data.id}</Eyebrow>
            <StatusDot status={open ? "open" : data.status === "open" ? "closing" : data.status} />
          </div>
          <h1 className="serif mt-4 text-[40px] leading-[1.05] sm:text-[56px]">{data.title}</h1>
          {data.description && <p className="dropcap mt-6 max-w-[52ch] text-[17px] leading-[1.6] text-muted-foreground">{data.description}</p>}
        </div>

        <div className="rise rise-2 grid grid-cols-2 gap-8 sm:grid-cols-3">
          <Fact label="Reserve" large>
            {zec(data.reserve_zats)}
          </Fact>
          <Fact label="Deposit to bid" large>
            {zec(data.deposit_zats)}
          </Fact>
          <Fact label={open ? "Closes in" : "Closed"} large>
            <span aria-live="polite">{open ? countdown(remaining) : dateTime(data.close_at)}</span>
          </Fact>
        </div>

        <div className="rise rise-3 hairline-t flex flex-col gap-8 pt-8">
          <div className="grid gap-6 sm:grid-cols-2">
            <div>
              <Eyebrow>Bids received</Eyebrow>
              <p className="num mt-1.5 text-[15px]">{data.bid_count}</p>
              <p className="mt-1 text-[13px] text-muted-foreground">Count only. Amounts stay sealed until close.</p>
            </div>
            <div>
              <Eyebrow>Auction address</Eyebrow>
              <Mono className="mt-1.5 block text-muted-foreground">{data.address}</Mono>
            </div>
          </div>
          <div className="grid gap-6 text-[14px] leading-relaxed text-muted-foreground sm:grid-cols-3">
            <p>
              <span className="text-foreground">1. Seal.</span> Your bid is written into the encrypted memo of one shielded
              payment of the deposit.
            </p>
            <p>
              <span className="text-foreground">2. Close.</span> Highest bid at or above reserve wins. A hash of the bid
              ledger is published.
            </p>
            <p>
              <span className="text-foreground">3. Settle.</span> Losers are refunded shielded. The winner pays the balance.
              Only the winning output is proven.
            </p>
          </div>
        </div>

        <div className="rise rise-4 hairline-t pt-8">
          <EventTimeline auctionId={data.id} />
        </div>

        <p className="text-[13px] text-muted-foreground">
          Already bid?{" "}
          <Link href={`/a/${data.id}/status`} className="text-foreground underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">
            Check your receipt
          </Link>
          .
        </p>
      </div>

      <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
        {open ? (
          <BidPanel auction={data} />
        ) : (
          <Panel className="flex flex-col items-start gap-5">
            <p className="serif text-[28px]">Bidding has closed.</p>
            <p className="text-muted-foreground">The result page shows the winning amount, the ledger commitment and the proof.</p>
            <LinkButton href={`/a/${data.id}/result`}>See the result</LinkButton>
          </Panel>
        )}
      </aside>
    </div>
    </div>
  );
}
