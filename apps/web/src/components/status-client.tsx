"use client";

import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";
import { parseZec, verifyInclusion } from "@blind-bazaar/sdk";
import { StandingPanel, savedBids } from "@/components/standing-panel";
import { ImportBidKey } from "@/components/import-bid-key";
import { zec } from "@/lib/format";
import { Button, Eyebrow, Field, LinkButton, Mono, Panel, Skeleton, inputClass } from "@/components/ui";
import { LotImage } from "@/components/lot-plate";

export function StatusClient({ id }: { id: string }) {
  const [receipt, setReceipt] = useState("");
  const [mine, setMine] = useState<{ receipt: string; nonce?: string; bid?: string }[]>([]);
  const [bidInput, setBidInput] = useState("");
  const [nonceInput, setNonceInput] = useState("");
  const [inclusion, setInclusion] = useState<{ ok: boolean; leafCount: number; root: string } | { error: string } | null>(null);
  useEffect(() => {
    try {
      const all = JSON.parse(localStorage.getItem("bb.receipts") ?? "{}") as Record<string, { receipt: string; nonce?: string; bid?: string }[]>;
      setMine(all[id] ?? []);
    } catch {
      /* no storage */
    }
  }, [id]);
  const { data, isPending, isError } = useQuery({
    queryKey: ["receipts", id],
    queryFn: () => api.receipts(id),
    refetchInterval: 10_000,
  });

  const lot = useQuery({ queryKey: ["auction", id], queryFn: () => api.auction(id) });
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const lookup = receipt.trim().toLowerCase();
  const found = data?.receipts.includes(lookup);
  const closed = lot.data ? lot.data.status !== "open" : false;
  const rememberedBid = mine.find((m) => m.receipt === lookup)?.bid;
  const rememberedNonce = mine.find((m) => m.receipt === lookup)?.nonce;

  async function checkInclusion() {
    setInclusion(null);
    let bidZats: bigint;
    try {
      bidZats = rememberedBid ? BigInt(rememberedBid) : parseZec(bidInput.trim());
    } catch {
      bidZats = 0n;
    }
    if (bidZats <= 0n) {
      setInclusion({ error: "Enter the bid amount in ZEC you sealed with this receipt." });
      return;
    }
    const nonce = (rememberedNonce ?? nonceInput.trim().toLowerCase());
    if (!/^[0-9a-f]{16}$/.test(nonce)) {
      setInclusion({ error: "Enter the 16-character nonce from your bid memo." });
      return;
    }
    try {
      const r = await api.proof(id, lookup);
      const ok = await verifyInclusion(lookup, bidZats, nonce, r.proof);
      setInclusion({ ok, leafCount: r.leaf_count, root: r.ledger_hash });
    } catch (e) {
      setInclusion({ error: e instanceof ApiError ? e.message : "Could not fetch the proof." });
    }
  }

  return (
    <div className="flex flex-col gap-10 sm:gap-14">
    <div className="plate rise">
      <LotImage id={id} src={lot.data?.image_url} alt={lot.data?.title ?? ""} ratio="16/5" />
    </div>
    <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
      <div className="flex flex-col gap-8 sm:col-span-7">
        <div className="rise">
          <Eyebrow>Lot {id}</Eyebrow>
          {lot.data && <p className="serif mt-4 text-[22px] text-muted-foreground">{lot.data.title}</p>}
          <h1 className="serif mt-2 text-[40px] leading-[1.05] sm:text-[56px]">Did my bid arrive?</h1>
          <p className="mt-6 max-w-[52ch] text-[16px] leading-relaxed text-muted-foreground">
            When you bid, this site gave you a receipt. The auction publishes the receipt of every bid it has read.
            If yours is in the list, you are in. A receipt reveals nothing about the amount or the bidder. After
            close, you can also check that your bid was counted in the published fingerprint.
          </p>
        </div>
        <div className="rise rise-2 hairline-t pt-8">
          <div className="flex items-baseline justify-between">
            <Eyebrow>All receipts</Eyebrow>
            {data && <span className="num text-[13px] text-muted-foreground">{data.receipts.length}</span>}
          </div>
          {isPending ? (
            <Skeleton className="mt-4 h-16" />
          ) : isError ? (
            <p className="mt-4 text-muted-foreground">Could not load receipts.</p>
          ) : data.receipts.length === 0 ? (
            <p className="mt-4 text-muted-foreground">No bids received yet.</p>
          ) : (
            <ul className="mt-4 flex flex-col">
              {data.receipts.map((r, i) => (
                <li key={r} className={`py-2.5 ${i > 0 ? "hairline-t" : ""}`}>
                  <Mono className={mine.some((m) => m.receipt === r) ? "text-foreground" : "text-muted-foreground"}>{r}</Mono>
                  {mine.some((m) => m.receipt === r) && <span className="ml-3 text-[12px] text-success">yours · received</span>}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      <aside className="rise rise-2 flex flex-col gap-6 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
        <StandingPanel auctionId={id} network={health.data?.network ?? "test"} />
        <ImportBidKey auctionId={id} onImported={() => setMine(savedBids(id))} />
        <Panel className="flex flex-col gap-6">
          <Field label="Your receipt" htmlFor="receipt" hint="64 hex characters, shown when you created the payment request.">
            <input
              id="receipt"
              name="receipt"
              type="text"
              autoComplete="off"
              spellCheck={false}
              className={`${inputClass} num text-[13px]`}
              value={receipt}
              onChange={(e) => setReceipt(e.target.value)}
            />
          </Field>
          {lookup.length === 64 && data && (
            <p className={`text-[14px] ${found ? "text-success" : "text-muted-foreground"}`} role="status">
              {found ? "Received. You are in the auction." : "Not seen yet. Shielded transactions take a block or two."}
            </p>
          )}
          {lookup.length === 64 && found && closed && (
            <div className="hairline-t flex flex-col gap-3 pt-5">
              <Eyebrow>Prove your bid was counted</Eyebrow>
              <p className="text-[13px] leading-relaxed text-muted-foreground">
                The ledger committed at close is a Merkle tree over every valid bid. Your browser recomputes the root
                from your receipt, your bid, your nonce and the path the service returns. The service never learns
                anything new, and nobody else can turn a leaf back into an amount without your nonce.
              </p>
              {!rememberedNonce && (
                <Field label="Your nonce" htmlFor="bid-nonce" hint="The last field of your bid memo, 16 characters.">
                  <input id="bid-nonce" name="bid-nonce" type="text" autoComplete="off" spellCheck={false} className={`${inputClass} num`} value={nonceInput} onChange={(e) => setNonceInput(e.target.value)} />
                </Field>
              )}
              {!rememberedBid && (
                <Field label="Your bid (ZEC)" htmlFor="bid-zats" hint="The amount you sealed. Checked here; not sent to the server.">
                  <input id="bid-zats" name="bid-zats" type="text" inputMode="decimal" autoComplete="off" placeholder="0.25" className={`${inputClass} num`} value={bidInput} onChange={(e) => setBidInput(e.target.value)} />
                </Field>
              )}
              <Button variant="secondary" onClick={checkInclusion}>
                Check inclusion
              </Button>
              {inclusion && "error" in inclusion && <p className="text-[13px] text-danger" role="alert">{inclusion.error}</p>}
              {inclusion && "ok" in inclusion && (
                <p className={`text-[14px] ${inclusion.ok ? "text-success" : "text-danger"}`} role="status">
                  {inclusion.ok
                    ? `Included. Your bid${rememberedBid ? ` of ${zec(Number(rememberedBid))}` : ""} is one of ${inclusion.leafCount} leaves under root ${inclusion.root.slice(0, 12)}…`
                    : "The path does not reproduce the committed root. Either the bid amount is wrong or the ledger was altered."}
                </p>
              )}
            </div>
          )}
          {mine.length > 0 && (
            <div className="hairline-t pt-5">
              <Eyebrow>Saved in this browser</Eyebrow>
              <ul className="mt-3 flex flex-col gap-2">
                {mine.map((m) => (
                  <li key={m.receipt} className="flex items-baseline justify-between gap-3">
                    <button type="button" onClick={() => setReceipt(m.receipt)} className="num text-left text-[12px] underline decoration-hairline-strong underline-offset-4 hover:decoration-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent">
                      {m.receipt.slice(0, 16)}…
                    </button>
                    <span className={`text-[12px] ${data?.receipts.includes(m.receipt) ? "text-success" : "text-muted-foreground"}`}>
                      {data?.receipts.includes(m.receipt) ? "received" : "pending"}
                    </span>
                  </li>
                ))}
              </ul>
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
