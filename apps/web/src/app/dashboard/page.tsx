"use client";

import { useMutation, useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { useSellerKey } from "@/lib/seller-key";
import { zec } from "@/lib/format";
import { fmt } from "@/lib/formats";
import { LotImage } from "@/components/lot-plate";
import { Button, Eyebrow, Field, LinkButton, Panel, Skeleton, StatusDot, inputClass } from "@/components/ui";

export default function DashboardPage() {
  const [key, setKey, keyReady] = useSellerKey();
  const [draft, setDraft] = useState("");
  const me = useQuery({ queryKey: ["seller-me", key], queryFn: () => api.sellerMe(key), enabled: key.length > 0, retry: false });
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const demo = useMutation({ mutationFn: api.demoSeller, onSuccess: (r) => setKey(r.key) });
  const notify = useQuery({ queryKey: ["seller-notify", key], queryFn: () => api.sellerNotifyGet(key), enabled: me.isSuccess });
  const [sellerEmail, setSellerEmail] = useState("");
  const setNotify = useMutation({ mutationFn: (email: string | null) => api.sellerNotifySet(key, email), onSuccess: () => notify.refetch() });
  const lots = useQuery({ queryKey: ["seller-lots", key], queryFn: () => api.sellerLots(key), enabled: me.isSuccess, refetchInterval: 15_000 });

  if (!keyReady) return <Skeleton className="h-64" />;
  if (!key || me.isError) {
    return (
      <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
        <div className="rise sm:col-span-7 min-w-0">
          <Eyebrow>Seller</Eyebrow>
          <h1 className="serif mt-4 text-[44px] leading-[1.02] sm:text-[64px]">Your lots, your money.</h1>
          <p className="dropcap mt-8 max-w-[46ch] text-[17px] leading-[1.6] text-muted-foreground">
            A seller key is issued by the operator and stays in this tab. From here you list lots, watch bids arrive
            as counts, see what has settled and what has been paid out, and leave delivery instructions that only the
            winner can read.
          </p>
        </div>
        <aside className="rise rise-2 sm:col-span-5 min-w-0">
          <Panel className="flex flex-col gap-5">
            <Field label="Seller key" htmlFor="seller-key" error={me.isError ? (me.error instanceof ApiError ? me.error.message : "Could not reach the service.") : undefined} hint="Starts with bbs_. Kept in this tab only. The operator issues keys; the admin token also works here.">
              <input id="seller-key" name="seller-key" type="password" autoComplete="off" className={`${inputClass} num`} value={draft} onChange={(e) => setDraft(e.target.value)} />
            </Field>
            <Button onClick={() => setKey(draft.trim())}>Open dashboard</Button>
            {health.data && health.data.network !== "main" && (
              <div className="hairline-t pt-4">
                <p className="text-[13px] text-muted-foreground">This is a test network. Try selling without asking anyone:</p>
                <Button variant="secondary" className="mt-3" onClick={() => demo.mutate()} disabled={demo.isPending}>
                  {demo.isPending ? "Creating…" : "Get a demo seller key"}
                </Button>
                {demo.isError && <p className="mt-2 text-[13px] text-danger">Could not create a demo key.</p>}
              </div>
            )}
          </Panel>
        </aside>
      </div>
    );
  }

  const open = lots.data?.filter((l) => l.status === "open") ?? [];
  const closed = lots.data?.filter((l) => l.status !== "open") ?? [];
  const totals = (lots.data ?? []).reduce(
    (t, l) => ({ paid: t.paid + l.money.seller_paid_zats, pending: t.pending + l.money.seller_pending_zats, held: t.held + l.money.deposits_held_zats }),
    { paid: 0, pending: 0, held: 0 },
  );

  return (
    <div className="flex flex-col gap-14">
      <div className="flex flex-col gap-6 sm:flex-row sm:items-end sm:justify-between">
        <div className="rise">
          <Eyebrow>Seller · {me.data?.name}</Eyebrow>
          <h1 className="serif mt-3 text-[40px] leading-none sm:text-[56px]">Dashboard</h1>
        </div>
        <div className="rise rise-2 flex items-center gap-3">
          <LinkButton href="/sell">List a lot</LinkButton>
          <Button variant="ghost" onClick={() => setKey("")}>Sign out</Button>
        </div>
      </div>
      <div className="flex flex-col gap-2 rounded-md border border-hairline p-5 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="text-[14px]">Email when a lot closes, a winner pays, or you are paid.</p>
          <p className="text-[12px] text-muted-foreground">{notify.data?.subscribed ? "On. Clear the field to stop." : "Optional. Stored only to send these mails."}</p>
        </div>
        <div className="flex gap-2">
          <input type="email" placeholder="you@example.com" className={`${inputClass} w-64 text-[14px]`} value={sellerEmail} onChange={(e) => setSellerEmail(e.target.value)} />
          <Button variant="secondary" disabled={setNotify.isPending} onClick={() => setNotify.mutate(sellerEmail.trim() || null)}>
            {setNotify.isPending ? "Saving…" : sellerEmail.trim() ? "Notify me" : "Stop"}
          </Button>
        </div>
      </div>

      <div className="rise rise-2 grid grid-cols-2 gap-8 sm:grid-cols-4">
        <Stat label="Paid to you" value={zec(totals.paid)} />
        <Stat label="Pending payout" value={zec(totals.pending)} />
        <Stat label="Deposits held" value={zec(totals.held)} hint="Bidders' money, not yours." />
        <Stat label="Open lots" value={String(open.length)} />
      </div>

      {lots.isPending ? (
        <Skeleton className="h-40" />
      ) : (lots.data ?? []).length === 0 ? (
        <p className="text-muted-foreground">No lots yet. List your first one.</p>
      ) : (
        <>
          <LotTable title="Open" lots={open} />
          <LotTable title="Closed" lots={closed} />
        </>
      )}
    </div>
  );
}

function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div>
      <Eyebrow>{label}</Eyebrow>
      <p className="serif mt-1.5 text-[28px] leading-none">{value}</p>
      {hint && <p className="mt-1 text-[12px] text-muted-foreground">{hint}</p>}
    </div>
  );
}

function LotTable({ title, lots }: { title: string; lots: import("@/lib/api").SellerLot[] }) {
  if (lots.length === 0) return null;
  return (
    <section className="flex flex-col">
      <div className="hairline-b flex items-baseline justify-between pb-3">
        <h2 className="serif text-[26px]">{title}</h2>
        <span className="num text-[13px] text-muted-foreground">{lots.length}</span>
      </div>
      <ul>
        {lots.map((l) => (
          <li key={l.id} className="hairline-b">
            <Link href={`/dashboard/${l.id}`} className="group grid items-center gap-4 py-4 transition-colors duration-150 hover:bg-foreground/[0.03] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent sm:grid-cols-[minmax(0,5fr)_repeat(4,minmax(0,1.5fr))_minmax(0,1fr)] sm:px-2">
              <div className="flex items-center gap-4">
                <div className="plate w-16 shrink-0"><LotImage id={l.id} src={l.image_url} alt="" ratio="1/1" /></div>
                <div className="min-w-0">
                  <p className="serif truncate text-[20px] leading-tight group-hover:text-accent">{l.title}</p>
                  <div className="mt-1 flex items-center gap-3"><StatusDot status={l.status} />{l.units > 1 ? <span className="eyebrow">Edition of {l.units}</span> : fmt(l.format).tag ? <span className="eyebrow">{fmt(l.format).tag}</span> : null}</div>
                </div>
              </div>
              <Cell label="Bids" value={String(l.money.bids_valid)} />
              <Cell label={l.status === "open" ? "Reserve" : "Clearing"} value={l.status === "open" ? zec(l.reserve_zats) : l.clearing_zats === null ? "No sale" : zec(l.clearing_zats)} />
              <Cell label="Settled" value={l.money.winners ? `${l.money.winners_settled}/${l.money.winners}` : "—"} />
              <Cell label="Paid out" value={zec(l.money.seller_paid_zats)} />
              <div className="sm:text-right"><span className={`text-[12px] ${l.fulfilment ? "text-success" : "text-muted-foreground"}`}>{l.fulfilment ? "delivery posted" : l.money.winners_settled > 0 ? "post delivery" : ""}</span></div>
            </Link>
          </li>
        ))}
      </ul>
    </section>
  );
}

function Cell({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span className="eyebrow sm:hidden">{label} </span>
      <span className="num whitespace-nowrap text-[14px]">{value}</span>
    </div>
  );
}
