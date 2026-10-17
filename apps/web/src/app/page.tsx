"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { api, type Auction } from "@/lib/api";
import { countdown, zec } from "@/lib/format";
import { fmt } from "@/lib/formats";
import { useNow } from "@/lib/use-now";
import { LotImage } from "@/components/lot-plate";
import { Button, Eyebrow, Skeleton, StatusDot } from "@/components/ui";
import { Glossary, NetworkNotice, StartHere, WhyTrust } from "@/components/explain";

export default function Home() {
  const { data, isPending, isError, refetch } = useQuery({
    queryKey: ["auctions"],
    queryFn: api.auctions,
    refetchInterval: 10_000,
  });
  const now = useNow();
  const open = data?.filter((a) => a.status === "open") ?? [];
  const closed = data?.filter((a) => a.status !== "open") ?? [];
  const featured = open[0];

  return (
    <div className="flex flex-col gap-20 sm:gap-28">
      <section className="grid gap-10 sm:grid-cols-12 sm:gap-10">
        <div className="sm:col-span-7">
          <Eyebrow className="rise">Private auctions on Zcash</Eyebrow>
          <h1 className="serif rise rise-2 mt-4 text-[48px] leading-[0.98] sm:text-[80px]">
            Sell to the highest bid.
            <br />
            <span className="italic text-muted-foreground">Nobody sees the others.</span>
          </h1>
          <p className="rise rise-3 mt-8 max-w-[48ch] text-[17px] leading-[1.6] text-muted-foreground">
            Blind Bazaar is an auction house where bids are private. A bidder pays a deposit from a Zcash wallet with
            the bid hidden inside the payment. When the auction closes, the highest bid wins, everyone else gets their
            deposit back, and nobody, not even the seller, ever learns the losing bids. The result is proven on the
            Zcash chain, so you do not have to trust us.
          </p>
          <div className="rise rise-4 mt-8">
            <NetworkNotice />
          </div>
        </div>
        <div className="rise rise-3 sm:col-span-5">
          {featured ? (
            <Link href={`/a/${featured.id}`} className="group block focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent">
              <div className="plate">
                <LotImage id={featured.id} src={featured.image_url} alt={featured.title} ratio="4/5" className="transition-transform duration-500 ease-out group-hover:scale-[1.02]" />
              </div>
              <div className="mt-4 flex items-baseline justify-between gap-4">
                <p className="serif text-[22px] leading-tight group-hover:text-accent">{featured.title}</p>
                <p className="num shrink-0 text-[13px] text-muted-foreground">{now === null ? "…" : countdown(featured.close_at - now)}</p>
              </div>
              <Eyebrow className="mt-1">Featured lot · {fmt(featured.format).floorLabel.toLowerCase()} {zec(featured.reserve_zats)}</Eyebrow>
            </Link>
          ) : (
            <div className="plate">
              <LotImage id="blind-bazaar" alt="" ratio="4/5" />
            </div>
          )}
        </div>
      </section>

      <StartHere />

      <section id="open" aria-labelledby="open-heading" className="rise rise-4 flex flex-col gap-8 scroll-mt-8">
        <div className="hairline-b flex items-baseline justify-between pb-4">
          <div>
            <h2 id="open-heading" className="serif text-[32px]">
              Open lots
            </h2>
            <p className="mt-1 text-[13px] text-muted-foreground">Bidding is open. Reserve is the lowest price the seller accepts; deposit is what you pay to bid.</p>
          </div>
          {data && <span className="num text-[13px] text-muted-foreground">{open.length}</span>}
        </div>
        {isPending ? (
          <div className="grid gap-10 sm:grid-cols-2">
            <Skeleton className="aspect-[4/3]" />
            <Skeleton className="aspect-[4/3]" />
          </div>
        ) : isError ? (
          <div className="flex flex-col items-start gap-4 py-6">
            <p className="text-muted-foreground">The auction service is unreachable.</p>
            <Button variant="secondary" onClick={() => refetch()}>
              Retry
            </Button>
          </div>
        ) : open.length === 0 ? (
          <p className="py-6 text-muted-foreground">Nothing is open right now. The next lot appears here the moment a seller lists it.</p>
        ) : (
          <Gallery lots={open} now={now} />
        )}
      </section>

      {closed.length > 0 && (
        <section id="closed" aria-labelledby="closed-heading" className="flex flex-col gap-8 scroll-mt-8">
          <div className="hairline-b flex items-baseline justify-between pb-4">
            <div>
              <h2 id="closed-heading" className="serif text-[32px]">
                Closed
              </h2>
              <p className="mt-1 text-[13px] text-muted-foreground">Open one to see the winning price, the bid fingerprint and the proof.</p>
            </div>
            <span className="num text-[13px] text-muted-foreground">{closed.length}</span>
          </div>
          <Gallery lots={closed} now={now} compact />
        </section>
      )}

      <WhyTrust />
      <Glossary only={["Reserve", "Deposit", "Bid", "Receipt", "Ledger commitment", "Proof", "Settlement", "Unified address"]} />
    </div>
  );
}

function Gallery({ lots, now, compact = false }: { lots: Auction[]; now: number | null; compact?: boolean }) {
  return (
    <ul className={`grid gap-x-10 gap-y-12 ${compact ? "sm:grid-cols-3" : "sm:grid-cols-2"}`}>
      {lots.map((a) => (
        <li key={a.id}>
          <Link href={`/a/${a.id}`} className="group block focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent">
            <div className="plate">
              <LotImage id={a.id} src={a.image_url} alt={a.title} className="transition-transform duration-500 ease-out group-hover:scale-[1.02]" />
            </div>
            <div className="mt-4 flex items-start justify-between gap-6">
              <div>
                <p className={`serif leading-tight transition-colors duration-150 group-hover:text-accent ${compact ? "text-[20px]" : "text-[26px]"}`}>{a.title}</p>
                <div className="mt-1.5 flex items-center gap-3">
                  <StatusDot status={a.status} />
                  {a.units > 1 ? <span className="eyebrow">Edition of {a.units}</span> : fmt(a.format).tag ? <span className="eyebrow">{fmt(a.format).tag}</span> : null}
                </div>
              </div>
              <div className="shrink-0 text-right">
                <Eyebrow>{a.status === "open" ? "Closes in" : "Closed"}</Eyebrow>
                <p className="num mt-1 text-[14px]">{now === null ? "…" : countdown(a.close_at - now)}</p>
              </div>
            </div>
            <dl className="mt-3 flex gap-8 text-[13px]">
              <div>
                <dt className="eyebrow">{fmt(a.format).floorLabel}</dt>
                <dd className="num mt-0.5">{zec(a.reserve_zats)}</dd>
              </div>
              <div>
                <dt className="eyebrow">{a.format === "rfq" ? "Bond" : "Deposit"}</dt>
                <dd className="num mt-0.5">{zec(a.deposit_zats)}</dd>
              </div>
              {a.status !== "open" && a.clearing_zats !== null && (
                <div>
                  <dt className="eyebrow">{a.format === "rfq" ? "Winning quote" : "Sold for"}</dt>
                  <dd className="num mt-0.5">{zec(a.clearing_zats)}</dd>
                </div>
              )}
            </dl>
          </Link>
        </li>
      ))}
    </ul>
  );
}
