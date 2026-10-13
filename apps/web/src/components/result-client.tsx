"use client";

import { useQuery } from "@tanstack/react-query";
import { api, explorerTxUrl } from "@/lib/api";
import { duration, zec } from "@/lib/format";
import { LotImage } from "@/components/lot-plate";
import { Eyebrow, Fact, LinkButton, Mono, Panel, Skeleton, StatusDot } from "@/components/ui";

export function ResultClient({ id }: { id: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const lot = useQuery({ queryKey: ["auction", id], queryFn: () => api.auction(id) });
  const { data, isPending, isError, error } = useQuery({
    queryKey: ["result", id],
    queryFn: () => api.result(id),
    refetchInterval: 15_000,
    retry: false,
  });
  const network = health.data?.network ?? "test";

  if (isPending) {
    return (
      <div className="flex flex-col gap-10">
        <Skeleton className="aspect-[16/7]" />
        <Skeleton className="h-24 w-2/3" />
      </div>
    );
  }
  if (isError) {
    return (
      <div className="flex flex-col items-start gap-4">
        <p className="serif text-[28px]">{error.message === "auction is still open" ? "This lot is still open." : "No result yet."}</p>
        <LinkButton href={`/a/${id}`} variant="secondary">
          Back to the lot
        </LinkButton>
      </div>
    );
  }

  const sent = data.refunds.filter((r) => r.kind === "refund" && r.status === "sent").length;
  const pending = data.refunds.filter((r) => r.kind === "refund" && r.status !== "sent").length;
  const disclosure = (Array.isArray(data.disclosure) ? data.disclosure[0] : data.disclosure) as { verified_against_chain?: boolean; cmx?: string; action_index?: number } | null;
  const sold = data.winning_amount_zats !== null;
  const uniform = data.format === "uniform";
  const disclosures = Array.isArray(data.disclosure) ? (data.disclosure as { verified_against_chain?: boolean }[]) : data.disclosure ? [data.disclosure as { verified_against_chain?: boolean }] : [];
  const verifiedCount = disclosures.filter((d) => d.verified_against_chain).length;

  return (
    <div className="flex flex-col gap-10 sm:gap-14">
      <div className="plate rise relative">
        <LotImage id={id} src={lot.data?.image_url} alt={lot.data?.title ?? ""} ratio="16/7" className={sold ? "" : "opacity-60"} />
        <div className="absolute bottom-0 left-0 flex items-end gap-5 p-5 sm:p-8">
          <span className="rounded-[2px] bg-background/90 px-3 py-2 text-[11px] uppercase tracking-[0.14em] text-foreground backdrop-blur">
            {sold ? "Sold" : "Unsold"}
          </span>
        </div>
      </div>

      <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
        <div className="flex flex-col gap-10 sm:col-span-7">
          <div className="rise">
            <div className="flex items-center gap-4">
              <Eyebrow>Lot {id} · result</Eyebrow>
              <StatusDot status={data.status} />
            </div>
            {lot.data && <p className="serif mt-4 text-[26px] leading-tight text-muted-foreground">{lot.data.title}</p>}
            <h1 className="serif mt-2 text-[56px] leading-[0.95] sm:text-[96px]">{sold ? zec(data.winning_amount_zats!) : "No sale."}</h1>
            <p className="dropcap mt-8 max-w-[52ch] text-[17px] leading-[1.6] text-muted-foreground">
              {sold
                ? uniform
                  ? `${data.units_sold} of ${data.units} units sold at one price, the lowest winning bid among ${data.valid_bid_count} sealed bids. Winners above the clearing price pay only the clearing price. Losing amounts and bidders are never revealed. Every losing deposit returns shielded to the address its bidder sealed into the memo.`
                  : `The highest of ${data.valid_bid_count} sealed bids. The losing amounts and bidders are never revealed, not even to the seller. Every losing deposit returns shielded to the address its bidder sealed into the memo.`
                : "No bid met the reserve. Every deposit is being returned shielded to the address its bidder sealed into the memo. Nobody learns who bid, or how much."}
            </p>
          </div>
          <div className="rise rise-2 grid grid-cols-2 gap-8 sm:grid-cols-3">
            <Fact label="Sealed bids" large>
              {data.valid_bid_count}
            </Fact>
            {uniform && (
              <Fact label="Units sold" large>
                {data.units_sold}
                <span className="text-muted-foreground"> / {data.units}</span>
              </Fact>
            )}
            <Fact label={data.refund_latency_secs !== null ? "Refunded within" : "Refunds"} large>
              {data.refund_latency_secs !== null ? duration(data.refund_latency_secs) : sent}
              {data.refund_latency_secs === null && pending > 0 && <span className="text-muted-foreground"> +{pending} pending</span>}
            </Fact>
            <Fact label="Settlement">
              {data.payout_txid
                ? "Seller paid"
                : sold
                  ? `${data.units_settled} of ${data.units_sold} paid`
                  : "—"}
            </Fact>
          </div>
          <div className="rise rise-3 hairline-t flex flex-col gap-6 pt-8">
            {data.winning_txids.length > 0 && (
              <div>
                <Eyebrow>{data.winning_txids.length > 1 ? "Winning transactions" : "Winning transaction"}</Eyebrow>
                <ul className="mt-2 flex flex-col gap-1.5">
                  {data.winning_txids.map((t) => (
                    <li key={t}>
                      <a
                        href={explorerTxUrl(network, t)}
                        target="_blank"
                        rel="noreferrer"
                        className="num block break-all text-[13px] underline decoration-hairline-strong underline-offset-4 hover:decoration-accent"
                      >
                        {t}
                      </a>
                    </li>
                  ))}
                </ul>
                <p className="mt-1 text-[13px] text-muted-foreground">Indistinguishable on the explorer from any other shielded transaction. Only the proof ties them to this lot.</p>
              </div>
            )}
            <div>
              <Eyebrow>Ledger commitment at close</Eyebrow>
              <Mono className="mt-2 block text-muted-foreground">{data.ledger_hash ?? "—"}</Mono>
              <p className="mt-1 text-[13px] text-muted-foreground">A hash of every valid bid at the moment of close, published before any refund leaves.</p>
            </div>
          </div>
        </div>
        <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
          <Panel className="flex flex-col gap-5">
            <div>
              <Eyebrow>Proof</Eyebrow>
              <p className="serif mt-2 text-[32px] leading-tight">
                {disclosures.length > 0
                  ? verifiedCount === disclosures.length
                    ? disclosures.length > 1
                      ? `All ${disclosures.length} verified on chain.`
                      : "Verified on chain."
                    : `${verifiedCount} of ${disclosures.length} verified.`
                  : sold
                    ? "Pending."
                    : "Nothing to prove."}
              </p>
            </div>
            <p className="text-[14px] leading-relaxed text-muted-foreground">
              A note-opening disclosure of the winning output: the recipient, value and randomness that reproduce the
              on-chain commitment. Anyone can check it with the service binary and a public lightwalletd. Losing bids
              stay sealed.
            </p>
            {disclosures.length > 0 && (
              <div className="hairline-t pt-4">
                <Eyebrow>{disclosures.length > 1 ? `${disclosures.length} note openings` : `Commitment · action ${disclosure?.action_index}`}</Eyebrow>
                {disclosures.length === 1 && <Mono className="mt-2 block text-muted-foreground">{disclosure?.cmx}</Mono>}
                <details className="mt-4 text-[13px]">
                  <summary className="cursor-pointer text-muted-foreground hover:text-foreground">Full disclosure JSON</summary>
                  <pre className="num mt-3 overflow-x-auto text-[11px] leading-relaxed text-muted-foreground">{JSON.stringify(data.disclosure, null, 2)}</pre>
                </details>
              </div>
            )}
            <LinkButton href={`/a/${id}`} variant="secondary">
              Back to the lot
            </LinkButton>
          </Panel>
        </aside>
      </div>
    </div>
  );
}
