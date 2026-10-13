"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { api } from "@/lib/api";
import { countdown, zec } from "@/lib/format";
import { Button, Card, Skeleton, StatusPill } from "@/components/ui";

export default function Home() {
  const { data, isPending, isError, refetch } = useQuery({
    queryKey: ["auctions"],
    queryFn: api.auctions,
    refetchInterval: 10_000,
  });
  const now = Math.floor(Date.now() / 1000);

  return (
    <div className="flex flex-col gap-8">
      <section className="flex flex-col gap-3">
        <h1 className="text-3xl font-semibold tracking-tight">Private auctions on Zcash</h1>
        <p className="max-w-2xl text-muted-foreground">
          Every bid is a shielded transaction with the bid sealed in its encrypted memo. Other bidders never learn
          what you bid, or that you bid at all. Losers are refunded, the winner is provable.
        </p>
      </section>

      <section aria-labelledby="auctions-heading" className="flex flex-col gap-4">
        <h2 id="auctions-heading" className="text-lg font-medium">
          Auctions
        </h2>
        {isPending ? (
          <div className="grid gap-4 sm:grid-cols-2">
            <Skeleton className="h-36" />
            <Skeleton className="h-36" />
          </div>
        ) : isError ? (
          <Card className="flex flex-col items-start gap-3">
            <p className="text-sm">Could not reach the auction service.</p>
            <Button variant="secondary" onClick={() => refetch()}>
              Retry
            </Button>
          </Card>
        ) : data.length === 0 ? (
          <Card className="text-sm text-muted-foreground">
            No auctions yet. The first one will appear here as soon as a seller lists a lot.
          </Card>
        ) : (
          <ul className="grid gap-4 sm:grid-cols-2">
            {data.map((a) => (
              <li key={a.id}>
                <Link
                  href={`/a/${a.id}`}
                  className="block h-full rounded-lg border border-border bg-card p-5 transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
                >
                  <div className="flex items-start justify-between gap-3">
                    <h3 className="font-medium">{a.title}</h3>
                    <StatusPill status={a.status} />
                  </div>
                  <dl className="mt-4 grid grid-cols-2 gap-y-1 text-sm">
                    <dt className="text-muted-foreground">Reserve</dt>
                    <dd className="text-right font-mono">{zec(a.reserve_zats)}</dd>
                    <dt className="text-muted-foreground">Deposit</dt>
                    <dd className="text-right font-mono">{zec(a.deposit_zats)}</dd>
                    <dt className="text-muted-foreground">{a.status === "open" ? "Closes in" : "Closed"}</dt>
                    <dd className="text-right font-mono">{countdown(a.close_at - now)}</dd>
                  </dl>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
