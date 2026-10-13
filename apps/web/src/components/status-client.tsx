"use client";

import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { Card, Field, LinkButton, Mono, Skeleton, inputClass } from "@/components/ui";

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
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Bid status</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          The auction publishes the receipt of every bid it has received. Receipts reveal nothing about the bid.
        </p>
      </div>
      <Card className="flex flex-col gap-4">
        <Field label="Your receipt" htmlFor="receipt" hint="64 hex characters, shown when you created the payment request.">
          <input
            id="receipt"
            name="receipt"
            type="text"
            autoComplete="off"
            spellCheck={false}
            className={`${inputClass} font-mono`}
            value={receipt}
            onChange={(e) => setReceipt(e.target.value)}
          />
        </Field>
        {lookup.length === 64 && data && (
          <p className={`text-sm ${found ? "text-success" : "text-muted-foreground"}`} role="status">
            {found ? "Bid received. You are in the auction." : "Not seen yet. Shielded transactions take a block or two to arrive."}
          </p>
        )}
        {mine.length > 0 && (
          <div className="text-sm">
            <p className="text-muted-foreground">Receipts saved in this browser</p>
            <ul className="mt-1 flex flex-col gap-1">
              {mine.map((r) => (
                <li key={r} className="flex items-center justify-between gap-3">
                  <Mono>{r}</Mono>
                  <span className={data?.receipts.includes(r) ? "text-success" : "text-muted-foreground"}>
                    {data?.receipts.includes(r) ? "received" : "pending"}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </Card>
      <Card>
        <h2 className="text-sm font-medium">All receipts</h2>
        {isPending ? (
          <Skeleton className="mt-3 h-16" />
        ) : isError ? (
          <p className="mt-2 text-sm text-muted-foreground">Could not load receipts.</p>
        ) : data.receipts.length === 0 ? (
          <p className="mt-2 text-sm text-muted-foreground">No bids received yet.</p>
        ) : (
          <ul className="mt-2 flex flex-col gap-1">
            {data.receipts.map((r) => (
              <li key={r}>
                <Mono className="text-muted-foreground">{r}</Mono>
              </li>
            ))}
          </ul>
        )}
      </Card>
      <LinkButton href={`/a/${id}`} variant="secondary" className="self-start">
        Back to auction
      </LinkButton>
    </div>
  );
}
