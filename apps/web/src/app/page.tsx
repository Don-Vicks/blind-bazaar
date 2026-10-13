"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { api } from "@/lib/api";
import { countdown, zec } from "@/lib/format";
import { useNow } from "@/lib/use-now";
import { Button, Eyebrow, Skeleton, StatusDot } from "@/components/ui";

export default function Home() {
  const { data, isPending, isError, refetch } = useQuery({
    queryKey: ["auctions"],
    queryFn: api.auctions,
    refetchInterval: 10_000,
  });
  const now = useNow();
  const open = data?.filter((a) => a.status === "open") ?? [];
  const closed = data?.filter((a) => a.status !== "open") ?? [];

  return (
    <div className="flex flex-col gap-16 sm:gap-24">
      <section className="grid gap-10 sm:grid-cols-12 sm:gap-8">
        <div className="sm:col-span-8">
          <Eyebrow className="rise">Sealed-bid auctions on Zcash</Eyebrow>
          <h1 className="serif rise rise-2 mt-4 text-[44px] leading-[1.02] sm:text-[72px]">
            Bid in the dark.
            <br />
            <span className="italic text-muted-foreground">Only the result comes to light.</span>
          </h1>
        </div>
        <div className="rise rise-3 flex flex-col gap-5 text-[15px] leading-relaxed text-muted-foreground sm:col-span-4 sm:pt-3">
          <p>
            Each bid is one shielded transaction. The amount travels inside its encrypted memo, readable only by the
            auction. Other bidders never learn what you bid, or that you bid at all.
          </p>
          <p>Losers are refunded shielded. The winner is proven on chain. Nothing else is ever revealed.</p>
          <Link
            href="/embed"
            className="text-foreground underline decoration-hairline-strong underline-offset-4 transition-colors duration-150 hover:decoration-accent"
          >
            No ZEC? Enter from Solana or Ethereum
          </Link>
        </div>
      </section>

      <section aria-labelledby="open-heading" className="rise rise-4 flex flex-col">
        <div className="hairline-b flex items-baseline justify-between pb-4">
          <h2 id="open-heading" className="serif text-[28px]">
            Open lots
          </h2>
          {data && <span className="num text-[13px] text-muted-foreground">{open.length}</span>}
        </div>
        {isPending ? (
          <div className="flex flex-col gap-px pt-px">
            <Skeleton className="h-20" />
            <Skeleton className="h-20" />
          </div>
        ) : isError ? (
          <div className="flex flex-col items-start gap-4 py-10">
            <p className="text-muted-foreground">The auction service is unreachable.</p>
            <Button variant="secondary" onClick={() => refetch()}>
              Retry
            </Button>
          </div>
        ) : open.length === 0 ? (
          <p className="py-10 text-muted-foreground">
            Nothing is open right now. The next lot appears here the moment a seller lists it.
          </p>
        ) : (
          <LotList lots={open} now={now} />
        )}
      </section>

      {closed.length > 0 && (
        <section aria-labelledby="closed-heading" className="flex flex-col">
          <div className="hairline-b flex items-baseline justify-between pb-4">
            <h2 id="closed-heading" className="serif text-[28px]">
              Closed
            </h2>
            <span className="num text-[13px] text-muted-foreground">{closed.length}</span>
          </div>
          <LotList lots={closed} now={now} />
        </section>
      )}
    </div>
  );
}

function LotList({
  lots,
  now,
}: {
  lots: { id: string; title: string; reserve_zats: number; deposit_zats: number; close_at: number; status: string }[];
  now: number | null;
}) {
  return (
    <ul>
      {lots.map((a) => (
        <li key={a.id} className="hairline-b">
          <Link
            href={`/a/${a.id}`}
            className="group grid items-baseline gap-2 py-6 transition-colors duration-150 hover:bg-foreground/[0.03] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent sm:grid-cols-12 sm:gap-6 sm:px-2"
          >
            <div className="sm:col-span-6">
              <p className="serif text-[22px] leading-tight transition-colors duration-150 group-hover:text-accent sm:text-[26px]">
                {a.title}
              </p>
              <div className="mt-1.5">
                <StatusDot status={a.status} />
              </div>
            </div>
            <div className="sm:col-span-2">
              <Eyebrow>Reserve</Eyebrow>
              <p className="num mt-1">{zec(a.reserve_zats)}</p>
            </div>
            <div className="sm:col-span-2">
              <Eyebrow>Deposit</Eyebrow>
              <p className="num mt-1">{zec(a.deposit_zats)}</p>
            </div>
            <div className="sm:col-span-2 sm:text-right">
              <Eyebrow>{a.status === "open" ? "Closes in" : "Closed"}</Eyebrow>
              <p className="num mt-1">{now === null ? "…" : countdown(a.close_at - now)}</p>
            </div>
          </Link>
        </li>
      ))}
    </ul>
  );
}
