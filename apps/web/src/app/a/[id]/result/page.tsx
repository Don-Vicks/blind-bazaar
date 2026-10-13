"use client";

import { useQuery } from "@tanstack/react-query";
import { use } from "react";
import { api, explorerTxUrl } from "@/lib/api";
import { zec } from "@/lib/format";
import { Card, LinkButton, Mono, Skeleton, StatusPill } from "@/components/ui";

export default function ResultPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
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
      <Card className="flex flex-col items-start gap-3">
        <p className="text-sm">{error.message === "auction is still open" ? "This auction is still open." : "Result unavailable."}</p>
        <LinkButton href={`/a/${id}`} variant="secondary">
          Back to auction
        </LinkButton>
      </Card>
    );
  }

  const sent = data.refunds.filter((r) => r.kind === "refund" && r.status === "sent").length;
  const pending = data.refunds.filter((r) => r.kind === "refund" && r.status !== "sent").length;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-semibold tracking-tight">Result</h1>
        <StatusPill status={data.status} />
      </div>
      <Card>
        <dl className="grid gap-x-6 gap-y-3 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-muted-foreground">Winning bid</dt>
            <dd className="font-mono text-base">{data.winning_amount_zats === null ? "No bid met the reserve" : zec(data.winning_amount_zats)}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Valid bids</dt>
            <dd className="font-mono text-base">{data.valid_bid_count}</dd>
          </div>
          {data.winning_txid && (
            <div className="sm:col-span-2">
              <dt className="text-muted-foreground">Winning transaction</dt>
              <dd>
                <a
                  href={explorerTxUrl(network, data.winning_txid)}
                  target="_blank"
                  rel="noreferrer"
                  className="font-mono text-xs underline underline-offset-2 hover:text-foreground"
                >
                  {data.winning_txid}
                </a>
              </dd>
            </div>
          )}
          <div className="sm:col-span-2">
            <dt className="text-muted-foreground">Ledger commitment at close</dt>
            <dd>
              <Mono>{data.ledger_hash ?? "—"}</Mono>
            </dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Refunds</dt>
            <dd className="text-base">
              {sent} sent{pending > 0 ? `, ${pending} pending` : ""}
            </dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Settlement</dt>
            <dd className="text-base">
              {data.payout_txid ? "Seller paid" : data.settlement_txid ? "Winner paid, seller payout queued" : "Awaiting winner"}
            </dd>
          </div>
        </dl>
      </Card>
      <Card>
        <h2 className="text-sm font-medium">Payment disclosure</h2>
        {data.disclosure ? (
          <pre className="mt-2 overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">{JSON.stringify(data.disclosure, null, 2)}</pre>
        ) : (
          <p className="mt-2 text-sm text-muted-foreground">
            A note-opening proof of the winning transaction is published here after close. Losing bids are never
            disclosed.
          </p>
        )}
      </Card>
      <LinkButton href={`/a/${id}`} variant="secondary" className="self-start">
        Back to auction
      </LinkButton>
    </div>
  );
}
