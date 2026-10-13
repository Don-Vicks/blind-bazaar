"use client";

import { useQuery } from "@tanstack/react-query";
import { api, explorerTxUrl } from "@/lib/api";
import { zec } from "@/lib/format";
import { Eyebrow, Fact, LinkButton, Mono, Panel, Skeleton, StatusDot } from "@/components/ui";

export function ResultClient({ id }: { id: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const { data, isPending, isError, error } = useQuery({
    queryKey: ["result", id],
    queryFn: () => api.result(id),
    refetchInterval: 15_000,
    retry: false,
  });
  const network = health.data?.network ?? "test";

  if (isPending) return <Skeleton className="h-64" />;
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
  const disclosure = data.disclosure as { verified_against_chain?: boolean; cmx?: string; action_index?: number } | null;

  return (
    <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
      <div className="flex flex-col gap-10 sm:col-span-7">
        <div className="rise">
          <div className="flex items-center gap-4">
            <Eyebrow>Lot {id} · result</Eyebrow>
            <StatusDot status={data.status} />
          </div>
          <h1 className="serif mt-4 text-[44px] leading-none sm:text-[72px]">
            {data.winning_amount_zats === null ? "No sale." : zec(data.winning_amount_zats)}
          </h1>
          <p className="mt-6 max-w-[52ch] text-[16px] leading-relaxed text-muted-foreground">
            {data.winning_amount_zats === null
              ? "No bid met the reserve. Every deposit is being returned shielded."
              : `The highest of ${data.valid_bid_count} sealed bids. The losing amounts and bidders are never revealed, not even to the seller.`}
          </p>
        </div>
        <div className="rise rise-2 grid grid-cols-2 gap-8 sm:grid-cols-3">
          <Fact label="Sealed bids">{data.valid_bid_count}</Fact>
          <Fact label="Refunds">{sent} sent{pending > 0 ? `, ${pending} pending` : ""}</Fact>
          <Fact label="Settlement">
            {data.payout_txid ? "Seller paid" : data.settlement_txid ? "Winner paid" : "Awaiting winner"}
          </Fact>
        </div>
        <div className="rise rise-3 hairline-t flex flex-col gap-6 pt-8">
          {data.winning_txid && (
            <div>
              <Eyebrow>Winning transaction</Eyebrow>
              <a
                href={explorerTxUrl(network, data.winning_txid)}
                target="_blank"
                rel="noreferrer"
                className="num mt-2 block break-all text-[13px] underline decoration-hairline-strong underline-offset-4 hover:decoration-accent"
              >
                {data.winning_txid}
              </a>
            </div>
          )}
          <div>
            <Eyebrow>Ledger commitment at close</Eyebrow>
            <Mono className="mt-2 block text-muted-foreground">{data.ledger_hash ?? "—"}</Mono>
          </div>
        </div>
      </div>
      <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
        <Panel className="flex flex-col gap-5">
          <div>
            <Eyebrow>Proof</Eyebrow>
            <p className="serif mt-2 text-[28px] leading-tight">
              {disclosure ? (disclosure.verified_against_chain ? "Verified on chain." : "Published.") : "Pending."}
            </p>
          </div>
          <p className="text-[14px] leading-relaxed text-muted-foreground">
            A note-opening disclosure of the winning output: recipient, value and randomness that reproduce the on-chain
            commitment. Anyone can check it against the chain. Losing bids stay sealed.
          </p>
          {disclosure && (
            <div className="hairline-t pt-4">
              <Eyebrow>Commitment · action {disclosure.action_index}</Eyebrow>
              <Mono className="mt-2 block text-muted-foreground">{disclosure.cmx}</Mono>
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
  );
}
