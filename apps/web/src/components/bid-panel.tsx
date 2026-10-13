"use client";

import { looksLikeShieldedAddress, parseZec, prepareBid, type PreparedBid } from "@blind-bazaar/sdk";
import { QRCodeSVG } from "qrcode.react";
import { useState } from "react";
import type { AuctionView } from "@/lib/api";
import { zec } from "@/lib/format";
import { Button, Card, Field, Mono, inputClass } from "@/components/ui";

const RECEIPTS_KEY = "bb.receipts";

export function rememberReceipt(auctionId: string, receipt: string, nonce: string) {
  try {
    const all = JSON.parse(localStorage.getItem(RECEIPTS_KEY) ?? "{}") as Record<string, { receipt: string; nonce: string }[]>;
    (all[auctionId] ??= []).push({ receipt, nonce });
    localStorage.setItem(RECEIPTS_KEY, JSON.stringify(all));
  } catch {
    /* private mode or storage blocked */
  }
}

export function BidPanel({ auction }: { auction: AuctionView }) {
  const [amount, setAmount] = useState("");
  const [refund, setRefund] = useState("");
  const [confirmed, setConfirmed] = useState(false);
  const [errors, setErrors] = useState<{ amount?: string; refund?: string; confirm?: string }>({});
  const [prepared, setPrepared] = useState<PreparedBid | null>(null);
  const [copied, setCopied] = useState<"uri" | "memo" | null>(null);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const next: typeof errors = {};
    let bidZats = 0n;
    try {
      bidZats = parseZec(amount);
      if (bidZats < BigInt(auction.reserve_zats)) next.amount = `Bid must be at least ${zec(auction.reserve_zats)}.`;
    } catch {
      next.amount = "Enter an amount in ZEC, up to 8 decimals.";
    }
    if (!looksLikeShieldedAddress(refund.trim())) next.refund = "Enter a unified address (starts with u1 or utest1).";
    if (!confirmed) next.confirm = "Confirm the refund address is yours.";
    setErrors(next);
    if (Object.keys(next).length) return;
    const p = await prepareBid(auction, bidZats, refund.trim());
    rememberReceipt(auction.id, p.receipt, p.bid.nonce);
    setPrepared(p);
  }

  async function copy(text: string, which: "uri" | "memo") {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(which);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      /* clipboard unavailable */
    }
  }

  if (prepared) {
    return (
      <Card className="flex flex-col gap-5">
        <div>
          <h2 className="text-lg font-medium">Pay the deposit to place your bid</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Scan with Zodl, Zingo or YWallet. Your bid of <span className="font-mono">{zec(prepared.bid.bidZats)}</span>{" "}
            travels inside the encrypted memo. Only the auction can read it.
          </p>
        </div>
        <div className="flex flex-col items-center gap-4 sm:flex-row sm:items-start">
          <div className="rounded-lg bg-white p-3">
            <QRCodeSVG value={prepared.uri} size={208} level="M" aria-label="Payment QR code" />
          </div>
          <div className="flex min-w-0 flex-1 flex-col gap-3 text-sm">
            <div>
              <p className="text-muted-foreground">Deposit</p>
              <p className="font-mono text-base">{zec(prepared.depositZats)}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Payment URI</p>
              <Mono className="text-muted-foreground">{prepared.uri}</Mono>
            </div>
            <div className="flex flex-wrap gap-2">
              <a
                href={prepared.uri}
                className="inline-flex min-h-10 items-center rounded-md bg-accent px-4 text-sm font-medium text-accent-foreground hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
              >
                Open in wallet
              </a>
              <Button variant="secondary" onClick={() => copy(prepared.uri, "uri")}>
                {copied === "uri" ? "Copied" : "Copy URI"}
              </Button>
              <Button variant="secondary" onClick={() => copy(prepared.memo, "memo")}>
                {copied === "memo" ? "Copied" : "Copy memo"}
              </Button>
            </div>
          </div>
        </div>
        <div className="rounded-md border border-border bg-muted p-4 text-sm">
          <p className="font-medium">Keep your receipt</p>
          <Mono className="mt-1 block">{prepared.receipt}</Mono>
          <p className="mt-2 text-xs text-muted-foreground">
            It appears on the status page once your bid lands. It reveals nothing about your bid. Saved in this browser.
          </p>
        </div>
        <Button variant="secondary" onClick={() => setPrepared(null)} className="self-start">
          Start over
        </Button>
      </Card>
    );
  }

  return (
    <Card>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-5">
        <div>
          <h2 className="text-lg font-medium">Place a sealed bid</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            You pay the fixed deposit now. If you win, you pay the difference. If you lose, the deposit comes back
            shielded to the address below.
          </p>
        </div>
        <Field label="Your bid (ZEC)" htmlFor="bid-amount" error={errors.amount} hint={`Reserve is ${zec(auction.reserve_zats)}.`}>
          <input
            id="bid-amount"
            name="bid"
            type="text"
            inputMode="decimal"
            autoComplete="off"
            placeholder="0.00"
            className={`${inputClass} font-mono`}
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            aria-invalid={!!errors.amount}
          />
        </Field>
        <Field
          label="Refund address"
          htmlFor="refund"
          error={errors.refund}
          hint="A unified address from your wallet. Refunds and nothing else go here."
        >
          <input
            id="refund"
            name="refund"
            type="text"
            autoComplete="off"
            spellCheck={false}
            placeholder="u1…"
            className={`${inputClass} font-mono`}
            value={refund}
            onChange={(e) => setRefund(e.target.value)}
            aria-invalid={!!errors.refund}
          />
        </Field>
        <div className="flex flex-col gap-1.5">
          <label className="flex min-h-10 items-center gap-3 text-sm">
            <input
              type="checkbox"
              name="confirm"
              className="size-4 accent-accent"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
              aria-invalid={!!errors.confirm}
            />
            I control this refund address.
          </label>
          {errors.confirm && (
            <p className="text-xs text-danger" role="alert">
              {errors.confirm}
            </p>
          )}
        </div>
        <Button type="submit" className="self-start">
          Create payment request
        </Button>
      </form>
    </Card>
  );
}
