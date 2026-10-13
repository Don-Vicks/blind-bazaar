"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { api } from "@/lib/api";
import { countdown, dateTime, zec } from "@/lib/format";
import { useNow } from "@/lib/use-now";
import { Card, LinkButton, Mono, Skeleton, StatusPill } from "@/components/ui";
import { BidPanel } from "@/components/bid-panel";
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
      <div className="flex flex-col gap-6">
        <Skeleton className="h-9 w-2/3" />
        <Skeleton className="h-40" />
        <Skeleton className="h-64" />
      </div>
    );
  }
  if (isError) {
    return (
      <Card className="flex flex-col items-start gap-3">
        <p className="text-sm">Auction not found.</p>
        <LinkButton href="/" variant="secondary">
          Back to auctions
        </LinkButton>
      </Card>
    );
  }

  const remaining = data.close_at - (now ?? data.close_at);
  const open = data.status === "open" && remaining > 0;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">{data.title}</h1>
          <StatusPill status={open ? "open" : data.status === "open" ? "closing" : data.status} />
        </div>
        {data.description && <p className="text-muted-foreground">{data.description}</p>}
      </div>

      <Card>
        <dl className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-muted-foreground">Reserve</dt>
            <dd className="font-mono text-base">{zec(data.reserve_zats)}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Deposit to bid</dt>
            <dd className="font-mono text-base">{zec(data.deposit_zats)}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{open ? "Closes in" : "Closed at"}</dt>
            <dd className="font-mono text-base" aria-live="polite">
              {open ? countdown(remaining) : dateTime(data.close_at)}
            </dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Bids received</dt>
            <dd className="font-mono text-base">{data.bid_count}</dd>
          </div>
          <div className="sm:col-span-2">
            <dt className="text-muted-foreground">Auction address</dt>
            <dd>
              <Mono>{data.address}</Mono>
            </dd>
          </div>
        </dl>
      </Card>

      {open ? (
        <BidPanel auction={data} />
      ) : (
        <Card className="flex flex-col items-start gap-3">
          <p className="text-sm">Bidding has closed.</p>
          <LinkButton href={`/a/${data.id}/result`}>See the result</LinkButton>
        </Card>
      )}

      <EventTimeline auctionId={data.id} />

      <p className="text-xs text-muted-foreground">
        Already bid?{" "}
        <Link href={`/a/${data.id}/status`} className="underline underline-offset-2 hover:text-foreground">
          Check your receipt
        </Link>
        .
      </p>
    </div>
  );
}
