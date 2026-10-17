"use client";

import { looksLikeShieldedAddress, parseZec, prepareBid, type PreparedBid } from "@blind-bazaar/sdk";
import { GoDark } from "@blind-bazaar/go-dark";
import { QRCodeSVG } from "qrcode.react";
import { useState } from "react";
import { API_BASE, type AuctionView } from "@/lib/api";
import { zec } from "@/lib/format";
import { fmt, isRfq } from "@/lib/formats";
import { Button, Eyebrow, Field, Mono, Panel, inputClass } from "@/components/ui";

const RECEIPTS_KEY = "bb.receipts";

export function rememberReceipt(auctionId: string, receipt: string, nonce: string, bidZats?: bigint) {
  try {
    const all = JSON.parse(localStorage.getItem(RECEIPTS_KEY) ?? "{}") as Record<string, { receipt: string; nonce: string; bid?: string }[]>;
    (all[auctionId] ??= []).push({ receipt, nonce, bid: bidZats?.toString() });
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
  const rfq = isRfq(auction.format);
  const f = fmt(auction.format);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const next: typeof errors = {};
    let bidZats = 0n;
    try {
      bidZats = parseZec(amount);
      if (rfq) {
        if (bidZats === 0n) next.amount = "A quote above zero.";
        else if (bidZats > BigInt(auction.reserve_zats)) next.amount = `At most the budget, ${zec(auction.reserve_zats)}.`;
      } else if (bidZats < BigInt(auction.reserve_zats)) next.amount = `At least ${zec(auction.reserve_zats)}.`;
    } catch {
      next.amount = "An amount in ZEC, up to 8 decimals.";
    }
    if (!looksLikeShieldedAddress(refund.trim())) next.refund = "A unified address, starting with u1 or utest1.";
    if (!confirmed) next.confirm = "Confirm the refund address is yours.";
    setErrors(next);
    if (Object.keys(next).length) return;
    const p = await prepareBid(auction, bidZats, refund.trim());
    rememberReceipt(auction.id, p.receipt, p.bid.nonce, p.bid.bidZats);
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
      <Panel className="flex flex-col gap-7">
        <div>
          <Eyebrow>Your sealed {f.verb}</Eyebrow>
          <p className="serif mt-2 text-[34px] leading-none">{zec(prepared.bid.bidZats)}</p>
          <p className="mt-3 text-[14px] leading-relaxed text-muted-foreground">
            Pay the <span className="num text-foreground">{zec(prepared.depositZats)}</span> {rfq ? "bond" : "deposit"} from Zodl, Zingo or
            YWallet. The {f.verb} travels inside the encrypted memo; only the auction can read it.
          </p>
        </div>
        <div className="flex flex-col items-center gap-5 sm:flex-row sm:items-start">
          <div className="rounded-[2px] bg-qr p-3">
            <QRCodeSVG value={prepared.uri} size={176} level="M" bgColor="transparent" fgColor="#1a1816" aria-label="Payment QR code" />
          </div>
          <div className="flex min-w-0 flex-1 flex-col gap-3">
            <a
              href={prepared.uri}
              className="inline-flex min-h-11 items-center justify-center rounded-[2px] bg-foreground px-5 text-[13px] font-medium tracking-[0.02em] text-background hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface"
            >
              Open in wallet
            </a>
            <div className="grid grid-cols-2 gap-2">
              <Button variant="secondary" onClick={() => copy(prepared.uri, "uri")}>
                {copied === "uri" ? "Copied" : "Copy URI"}
              </Button>
              <Button variant="secondary" onClick={() => copy(prepared.memo, "memo")}>
                {copied === "memo" ? "Copied" : "Copy memo"}
              </Button>
            </div>
          </div>
        </div>
        <div className="hairline-t pt-5">
          <Eyebrow>Receipt · keep this</Eyebrow>
          <Mono className="mt-2 block">{prepared.receipt}</Mono>
          <p className="mt-2 text-[13px] text-muted-foreground">
            Appears on the status page once your {f.verb} lands. Reveals nothing about it. Saved in this browser.
          </p>
        </div>
        <div className="hairline-t pt-5">
          <Eyebrow>No ZEC?</Eyebrow>
          <p className="mb-4 mt-2 text-[13px] leading-relaxed text-muted-foreground">
            Go Dark swaps the {rfq ? "bond" : "deposit"} from Solana or Ethereum into shielded ZEC at the auction address and relays your
            sealed {f.verb}. The rail never sees it.
          </p>
          <GoDark
            to={auction.address}
            amountZats={prepared.depositZats.toString()}
            label="Enter with Go Dark"
            envelope={{ auctionApi: API_BASE, auctionId: auction.id, payload: prepared.memo }}
          />
        </div>
        <Button variant="ghost" onClick={() => setPrepared(null)} className="self-start px-0">
          Start over
        </Button>
      </Panel>
    );
  }

  return (
    <Panel>
      <form onSubmit={onSubmit} noValidate className="flex flex-col gap-6">
        <div>
          <Eyebrow>Place a sealed {f.verb}</Eyebrow>
          <p className="mt-3 text-[14px] leading-relaxed text-muted-foreground">
            {rfq
              ? "You post the fixed bond now. Win, and the buyer pays your quote; you receive it plus the bond. Lose, and the bond returns shielded to the address below."
              : auction.format === "vickrey"
                ? "You pay the fixed deposit now. Win, and you pay the second-highest bid, never your own. Lose, and the deposit returns shielded to the address below."
                : "You pay the fixed deposit now. Win, and you pay the difference. Lose, and the deposit returns shielded to the address below."}
          </p>
        </div>
        <Field label={f.amountLabel} htmlFor="bid-amount" error={errors.amount} hint={`${f.floorLabel} ${zec(auction.reserve_zats)}`}>
          <div className="relative">
            <input
              id="bid-amount"
              name="bid"
              type="text"
              inputMode="decimal"
              autoComplete="off"
              placeholder="0.00"
              className={`${inputClass} num pr-14 text-[22px]`}
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              aria-invalid={!!errors.amount}
            />
            <span className="num pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-[13px] text-muted-foreground">ZEC</span>
          </div>
        </Field>
        <Field label="Refund address" htmlFor="refund" error={errors.refund} hint="From your wallet. Refunds and nothing else go here.">
          <input
            id="refund"
            name="refund"
            type="text"
            autoComplete="off"
            spellCheck={false}
            placeholder="u1…"
            className={`${inputClass} num text-[13px]`}
            value={refund}
            onChange={(e) => setRefund(e.target.value)}
            aria-invalid={!!errors.refund}
          />
        </Field>
        <div className="flex flex-col gap-1.5">
          <label className="flex min-h-11 items-center gap-3 text-[14px]">
            <input
              type="checkbox"
              name="confirm"
              className="size-4 accent-[var(--accent)]"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
              aria-invalid={!!errors.confirm}
            />
            I control this refund address.
          </label>
          {errors.confirm && (
            <p className="text-[13px] text-danger" role="alert">
              {errors.confirm}
            </p>
          )}
        </div>
        <Button type="submit" className="w-full">
          Create payment request
        </Button>
      </form>
    </Panel>
  );
}
