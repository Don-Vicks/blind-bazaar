"use client";

import { isSalt, isSecret, nonceFromSecret, parseZec, receipt } from "@blind-bazaar/sdk";
import { useEffect, useState } from "react";
import { Button, Eyebrow, Field, inputClass } from "@/components/ui";
import { parseCardFile, parseCardFragment, saveBidRecord } from "@/lib/bid-card";

/** Paste a bid key from another device so this browser can show that bid's standing. */
export function ImportBidKey({ auctionId, onImported }: { auctionId: string; onImported: () => void }) {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  const [salt, setSalt] = useState("");
  const [amount, setAmount] = useState("");
  const [imported, setImported] = useState<string | null>(null);

  // A bid-card link lands here with the key in the fragment: import it and clear the fragment.
  useEffect(() => {
    const frag = parseCardFragment(window.location.hash);
    if (!frag) return;
    (async () => {
      const nonce = await nonceFromSecret(frag.secret);
      const r = await receipt(nonce);
      const bid = (() => {
        if (!frag.bid) return undefined;
        try {
          return BigInt(frag.bid).toString();
        } catch {
          return undefined;
        }
      })();
      if (saveBidRecord(auctionId, { receipt: r, nonce, secret: frag.secret, bid, salt: frag.salt && isSalt(frag.salt) ? frag.salt : undefined })) {
        history.replaceState(null, "", window.location.pathname);
        setImported("Bid imported from your link.");
        onImported();
      }
    })();
  }, [auctionId, onImported]);

  async function fromFile(file: File) {
    const card = parseCardFile(await file.text());
    if (!card) {
      setError("That is not a Blind Bazaar bid card.");
      return;
    }
    if (card.auction_id !== auctionId) {
      setError(`This card is for lot ${card.auction_id}; open that lot's status page.`);
      return;
    }
    if (saveBidRecord(auctionId, { receipt: card.receipt, nonce: card.nonce, secret: card.secret, bid: card.bid, salt: card.salt })) {
      setImported("Bid card imported.");
      setOpen(false);
      onImported();
    } else {
      setError("This browser blocks storage; the card cannot be kept here.");
    }
  }
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
      const list = (all[auctionId] ??= []) as { receipt: string; nonce: string; secret: string; salt?: string; bid?: string }[];
      let bid: string | undefined;
      if (amount.trim()) {
        try {
          bid = parseZec(amount.trim()).toString();
        } catch {
          setError("The bid amount must be in ZEC.");
          return;
        }
      }
      const s = salt.trim().toLowerCase();
      if (s && !isSalt(s)) {
        setError("A reveal code is 32 hex characters.");
        return;
      }
      if (!list.some((b) => b.secret === secret)) list.push({ receipt: r, nonce, secret, salt: s || undefined, bid });
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
      <div className="flex flex-col gap-2">
        {imported && <p className="text-[13px] text-success">{imported}</p>}
        <button type="button" onClick={() => setOpen(true)} className="self-start text-[13px] text-muted-foreground underline decoration-hairline-strong underline-offset-4 hover:text-foreground">
          Bid from another device? Paste your bid key or open a bid card
        </button>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-3 rounded-md border border-hairline p-5">
      <Eyebrow>Bid key</Eyebrow>
      <Field label="Paste the key shown when you bid" htmlFor="bid-key" error={error ?? undefined} hint="Never sent to the auction in full; it derives your receipt and proves the bid is yours.">
        <input id="bid-key" className={`${inputClass} num text-[13px]`} spellCheck={false} value={value} onChange={(e) => setValue(e.target.value)} />
      </Field>
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="Bid amount (ZEC)" htmlFor="bid-key-amount" hint="Needed to reveal a sealed bid or check inclusion.">
          <input id="bid-key-amount" inputMode="decimal" className={`${inputClass} num`} value={amount} onChange={(e) => setAmount(e.target.value)} />
        </Field>
        <Field label="Reveal code" htmlFor="bid-key-salt" hint="Sealed lots only; shown with the bid key.">
          <input id="bid-key-salt" className={`${inputClass} num text-[13px]`} spellCheck={false} value={salt} onChange={(e) => setSalt(e.target.value)} />
        </Field>
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button onClick={add}>Add to this browser</Button>
        <label className="inline-flex min-h-11 cursor-pointer items-center rounded-[2px] border border-hairline-strong px-5 text-[13px] font-medium hover:bg-foreground/5">
          Open a bid card file
          <input type="file" accept="application/json,.json" className="sr-only" onChange={(e) => { const f = e.target.files?.[0]; if (f) fromFile(f); }} />
        </label>
        <Button variant="ghost" onClick={() => setOpen(false)}>Cancel</Button>
      </div>
    </div>
  );
}
