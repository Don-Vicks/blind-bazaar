"use client";

import { isSecret, nonceFromSecret, receipt } from "@blind-bazaar/sdk";
import { useState } from "react";
import { Button, Eyebrow, Field, inputClass } from "@/components/ui";

/** Paste a bid key from another device so this browser can show that bid's standing. */
export function ImportBidKey({ auctionId, onImported }: { auctionId: string; onImported: () => void }) {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  async function add() {
    const secret = value.trim().toLowerCase();
    if (!isSecret(secret)) {
      setError("A bid key is 32 hex characters.");
      return;
    }
    const nonce = await nonceFromSecret(secret);
    const r = await receipt(nonce);
    try {
      const all = JSON.parse(localStorage.getItem("bb.receipts") ?? "{}") as Record<string, { receipt: string; nonce: string; secret: string }[]>;
      const list = (all[auctionId] ??= []);
      if (!list.some((b) => b.secret === secret)) list.push({ receipt: r, nonce, secret });
      localStorage.setItem("bb.receipts", JSON.stringify(all));
    } catch {
      setError("This browser blocks storage; the key cannot be kept here.");
      return;
    }
    setValue("");
    setError(null);
    setOpen(false);
    onImported();
  }
  if (!open) {
    return (
      <button type="button" onClick={() => setOpen(true)} className="self-start text-[13px] text-muted-foreground underline decoration-hairline-strong underline-offset-4 hover:text-foreground">
        Bid from another device? Paste your bid key
      </button>
    );
  }
  return (
    <div className="flex flex-col gap-3 rounded-md border border-hairline p-5">
      <Eyebrow>Bid key</Eyebrow>
      <Field label="Paste the key shown when you bid" htmlFor="bid-key" error={error ?? undefined} hint="Never sent to the auction in full; it derives your receipt and proves the bid is yours.">
        <input id="bid-key" className={`${inputClass} num text-[13px]`} spellCheck={false} value={value} onChange={(e) => setValue(e.target.value)} />
      </Field>
      <div className="flex gap-3">
        <Button onClick={add}>Add to this browser</Button>
        <Button variant="ghost" onClick={() => setOpen(false)}>Cancel</Button>
      </div>
    </div>
  );
}
