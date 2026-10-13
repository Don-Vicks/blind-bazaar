"use client";

import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { Eyebrow, Field, LinkButton, Mono, Panel, Skeleton, inputClass } from "@/components/ui";

export function StatusClient({ id }: { id: string }) {
  const [receipt, setReceipt] = useState("");
  const [mine, setMine] = useState<string[]>([]);
  useEffect(() => {
    try {
      const all = JSON.parse(localStorage.getItem("bb.receipts") ?? "{}") as Record<string, { receipt: string }[]>;
      setMine((all[id] ?? []).map((r) => r.receipt));
    } catch {
      /* no storage */
    }
  }, [id]);
  const { data, isPending, isError } = useQuery({
    queryKey: ["receipts", id],
    queryFn: () => api.receipts(id),
    refetchInterval: 10_000,
  });

  const lookup = receipt.trim().toLowerCase();
  const found = data?.receipts.includes(lookup);

  return (
    <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
      <div className="flex flex-col gap-8 sm:col-span-7">
        <div className="rise">
          <Eyebrow>Lot {id}</Eyebrow>
          <h1 className="serif mt-4 text-[40px] leading-[1.05] sm:text-[56px]">Was my bid received?</h1>
          <p className="mt-6 max-w-[52ch] text-[16px] leading-relaxed text-muted-foreground">
            The auction publishes the receipt of every bid it has decrypted. A receipt is a hash of the nonce your
            browser generated; it reveals nothing about the amount or the bidder.
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
                  <Mono className={mine.includes(r) ? "text-foreground" : "text-muted-foreground"}>{r}</Mono>
                  {mine.includes(r) && <span className="ml-3 text-[12px] text-success">yours · received</span>}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
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
          {mine.length > 0 && (
            <div className="hairline-t pt-5">
              <Eyebrow>Saved in this browser</Eyebrow>
              <ul className="mt-3 flex flex-col gap-2">
                {mine.map((r) => (
                  <li key={r} className="flex items-baseline justify-between gap-3">
                    <Mono className="text-[12px]">{r.slice(0, 16)}…</Mono>
                    <span className={`text-[12px] ${data?.receipts.includes(r) ? "text-success" : "text-muted-foreground"}`}>
                      {data?.receipts.includes(r) ? "received" : "pending"}
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
  );
}
