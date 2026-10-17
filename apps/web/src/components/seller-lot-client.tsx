"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, explorerTxUrl } from "@/lib/api";
import { useSellerKey } from "@/lib/seller-key";
import { dateTime, zec } from "@/lib/format";
import { LotImage } from "@/components/lot-plate";
import { Button, Eyebrow, Fact, Field, LinkButton, Mono, Panel, Skeleton, StatusDot, inputClass } from "@/components/ui";

export function SellerLotClient({ id }: { id: string }) {
  const [key] = useSellerKey();
  const qc = useQueryClient();
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const { data, isPending, isError } = useQuery({ queryKey: ["seller-lot", key, id], queryFn: () => api.sellerLot(key, id), enabled: key.length > 0, refetchInterval: 15_000 });
  const [note, setNote] = useState("");
  useEffect(() => {
    if (data?.fulfilment && !note) setNote(data.fulfilment);
  }, [data?.fulfilment, note]);
  const save = useMutation({
    mutationFn: () => api.setFulfilment(key, id, note),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["seller-lot", key, id] }),
  });
  const network = health.data?.network ?? "test";

  if (!key) {
    return (
      <div className="flex flex-col items-start gap-4">
        <p className="serif text-[28px]">Sign in on the dashboard first.</p>
        <LinkButton href="/dashboard" variant="secondary">Dashboard</LinkButton>
      </div>
    );
  }
  if (isPending) return <Skeleton className="h-64" />;
  if (isError || !data) {
    return (
      <div className="flex flex-col items-start gap-4">
        <p className="serif text-[28px]">Not one of your lots.</p>
        <LinkButton href="/dashboard" variant="secondary">Back to dashboard</LinkButton>
      </div>
    );
  }
  const m = data.money;

  return (
    <div className="flex flex-col gap-10 sm:gap-14">
      <div className="plate rise"><LotImage id={id} src={data.image_url} alt={data.title} ratio="16/5" /></div>
      <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
        <div className="flex flex-col gap-10 sm:col-span-7">
          <div className="rise">
            <div className="flex items-center gap-4"><Eyebrow>Lot {id}</Eyebrow><StatusDot status={data.status} /></div>
            <h1 className="serif mt-4 text-[40px] leading-[1.05] sm:text-[56px]">{data.title}</h1>
          </div>
          <div className="rise rise-2 grid grid-cols-2 gap-8 sm:grid-cols-3">
            <Fact label="Sealed bids" large>{m.bids_valid}</Fact>
            <Fact label={data.status === "open" ? "Reserve" : "Clearing price"} large>{data.status === "open" ? zec(data.reserve_zats) : data.clearing_zats === null ? "No sale" : zec(data.clearing_zats)}</Fact>
            <Fact label="Paid to you" large>{zec(m.seller_paid_zats)}</Fact>
            <Fact label="Winners settled">{m.winners ? `${m.winners_settled} of ${m.winners}` : "—"}</Fact>
            <Fact label="Pending payout">{zec(m.seller_pending_zats)}</Fact>
            <Fact label="Refunds">{m.refunds_sent} sent{m.refunds_pending ? `, ${m.refunds_pending} pending` : ""}</Fact>
          </div>
          <div className="rise rise-3 hairline-t pt-8">
            <Eyebrow>Payouts</Eyebrow>
            {data.payouts.length === 0 ? (
              <p className="mt-3 text-[14px] text-muted-foreground">Nothing has moved yet.</p>
            ) : (
              <ul className="mt-3 flex flex-col">
                {data.payouts.map((p, i) => (
                  <li key={p.id} className={`grid grid-cols-[90px_1fr_auto] items-baseline gap-3 py-2.5 text-[13px] ${i > 0 ? "hairline-t" : ""}`}>
                    <span className="eyebrow">{p.kind}</span>
                    <span>{p.txid ? <a href={explorerTxUrl(network, p.txid)} target="_blank" rel="noreferrer" className="num underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">{p.txid.slice(0, 18)}…</a> : <span className="text-muted-foreground">{p.status}</span>}</span>
                    <span className="num">{zec(p.value_zats)}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <div className="rise rise-4 hairline-t pt-8">
            <Eyebrow>Timeline</Eyebrow>
            <ol className="mt-3 flex flex-col">
              {data.events.map((e, i) => (
                <li key={e.id} className={`grid grid-cols-[150px_1fr] gap-4 py-2.5 text-[13px] ${i > 0 ? "hairline-t" : ""}`}>
                  <span className="num text-[12px] text-muted-foreground">{dateTime(e.created_at)}</span>
                  <span>{e.kind.replace(/_/g, " ")}</span>
                </li>
              ))}
            </ol>
          </div>
        </div>
        <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
          <Panel className="flex flex-col gap-5">
            <div>
              <Eyebrow>Delivery</Eyebrow>
              <p className="serif mt-2 text-[26px] leading-tight">Only the winner reads this.</p>
            </div>
            <p className="text-[14px] leading-relaxed text-muted-foreground">
              A download link, a shipping handshake, a contact. A bidder sees it only by presenting a receipt that won
              this lot. Post it once a winner has settled.
            </p>
            <Field label="Note to the winner" htmlFor="note">
              <textarea id="note" name="note" rows={5} className={`${inputClass} min-h-32 py-2`} value={note} onChange={(e) => setNote(e.target.value)} />
            </Field>
            <Button onClick={() => save.mutate()} disabled={save.isPending || !note.trim()}>{save.isPending ? "Saving…" : data.fulfilment ? "Update note" : "Post note"}</Button>
            {save.isSuccess && <p className="text-[13px] text-success">Posted.</p>}
            {save.isError && <p className="text-[13px] text-danger">Could not save.</p>}
            <div className="hairline-t pt-4"><Eyebrow>Payout address</Eyebrow><Mono className="mt-2 block text-muted-foreground">{data.seller_address}</Mono></div>
            <LinkButton href={`/a/${id}`} variant="secondary">Public lot page</LinkButton>
          </Panel>
        </aside>
      </div>
    </div>
  );
}
