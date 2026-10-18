"use client";

import { useMutation, useQueries, useQueryClient } from "@tanstack/react-query";
import { QRCodeSVG } from "qrcode.react";
import { useEffect, useState } from "react";
import { GoDark } from "@blind-bazaar/go-dark";
import { API_BASE, ApiError, api, explorerTxUrl, type Standing } from "@/lib/api";
import { dateTime, zec } from "@/lib/format";
import { Button, Eyebrow, Mono, Panel } from "@/components/ui";

export interface SavedBid {
  receipt: string;
  nonce: string;
  /** Authenticates the bidder to the service. Absent for bids made before secrets existed. */
  secret?: string;
  bid?: string;
  /** Sealed-reveal lots: the salt behind the commitment. */
  salt?: string;
}

/** Bids this browser prepared for `auctionId`, from localStorage. */
export function savedBids(auctionId: string): SavedBid[] {
  try {
    const all = JSON.parse(localStorage.getItem("bb.receipts") ?? "{}") as Record<string, SavedBid[]>;
    return (all[auctionId] ?? []).filter((b) => b.nonce);
  } catch {
    return [];
  }
}

const OUTCOME: Record<string, { title: string; body: string }> = {
  pending: { title: "Your bid is in.", body: "It was read and counted. Nothing more to do until the lot closes." },
  sealed: { title: "Your sealed bid is in.", body: "Only its commitment is on record; nobody, including the auction, knows the amount. When the lot closes you reveal it here." },
  reveal_now: { title: "Reveal your bid now.", body: "The lot has closed. Reveal before the window ends or your deposit is returned and the bid cannot win." },
  revealed: { title: "Revealed.", body: "Your bid is counted. The result appears when the reveal window ends." },
  unrevealed: { title: "Not revealed in time.", body: "Your deposit is being returned in full. The bid could not take part." },
  won: { title: "You won.", body: "Pay the balance below from your Zcash wallet before the deadline. Then the seller is paid and the delivery note appears here." },
  settled: { title: "You won and paid.", body: "The seller has been paid or is being paid. The delivery note is below." },
  lost: { title: "Not this time.", body: "Your deposit is coming back to the refund address you sealed into your bid, as a private payment, after a short random delay. Nobody learns you bid." },
  forfeited: { title: "The deadline passed.", body: "The balance was not paid in time, so the deposit went to the seller and the lot was offered to the next bidder." },
  expired: { title: "The buyer did not pay.", body: "Your quote won, but the buyer never paid. Your bond is being returned and the request has lapsed." },
  rejected: { title: "This payment was not counted as a bid.", body: "It arrived late, with the wrong deposit, or with a memo the auction could not read. If it carried a refund address, the deposit is being returned there." },
};

/** What each bid this browser made stands at, and how to finish it. Authenticated by the nonce only this browser holds. */
export function StandingPanel({ auctionId, network }: { auctionId: string; network: "test" | "main" }) {
  const [mine, setMine] = useState<SavedBid[]>([]);
  useEffect(() => setMine(savedBids(auctionId)), [auctionId]);
  const withSecret = mine.filter((b) => b.secret);
  const results = useQueries({
    queries: withSecret.map((b) => ({
      queryKey: ["standing", auctionId, b.secret],
      queryFn: () => api.standing(auctionId, b.secret!),
      refetchInterval: 15_000,
      retry: false,
    })),
  });
  if (withSecret.length === 0) return null;
  const rows = withSecret.map((b, i) => ({ b, q: results[i] })).filter(({ q }) => !(q.isError && q.error instanceof ApiError && q.error.status === 404));
  if (rows.length === 0) return null;
  return (
    <div className="flex flex-col gap-6">
      {rows.map(({ b, q }) => (
        <StandingCard key={b.nonce} saved={b} standing={q.data} error={q.isError ? (q.error as Error).message : null} network={network} auctionId={auctionId} />
      ))}
    </div>
  );
}

function StandingCard({ saved, standing, error, network, auctionId }: { saved: SavedBid; standing?: Standing; error: string | null; network: "test" | "main"; auctionId: string }) {
  const [copied, setCopied] = useState(false);
  const qc = useQueryClient();
  if (error) return <Panel className="text-[14px] text-muted-foreground">Could not load your bid: {error}</Panel>;
  if (!standing) return <Panel className="text-[14px] text-muted-foreground">Checking your bid…</Panel>;
  const o = OUTCOME[standing.outcome] ?? OUTCOME.rejected;
  const rfq = standing.format === "rfq";
  const refund = standing.payouts.find((p) => p.kind === "refund");
  const reveal = useMutation({
    mutationFn: () => api.reveal(auctionId, saved.secret!, BigInt(saved.bid ?? "0"), saved.salt ?? ""),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["standing", auctionId, saved.secret] }),
  });
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable */
    }
  }
  return (
    <Panel className="flex flex-col gap-5">
      <div>
        <Eyebrow>Your {rfq ? "quote" : "bid"} · receipt {saved.receipt.slice(0, 10)}…</Eyebrow>
        <p className="serif mt-2 text-[30px] leading-tight">{rfq && standing.outcome === "won" ? "Your quote won." : o.title}</p>
        <p className="mt-2 text-[14px] leading-relaxed text-muted-foreground">
          {rfq && standing.outcome === "won"
            ? "The buyer pays your quote; you then receive it plus your bond, privately. Nothing to do on your side."
            : o.body}
        </p>
      </div>
      {standing.price_zats !== null && (standing.outcome === "won" || standing.outcome === "settled") && (
        <dl className="grid grid-cols-3 gap-4 text-[13px]">
          <div><dt className="eyebrow">{rfq ? "Your quote" : "Price"}</dt><dd className="num mt-1">{zec(standing.price_zats)}</dd></div>
          <div><dt className="eyebrow">{rfq ? "Bond" : "Deposit paid"}</dt><dd className="num mt-1">{zec(standing.deposit_zats)}</dd></div>
          <div><dt className="eyebrow">{rfq ? "You receive" : "Still owed"}</dt><dd className="num mt-1">{rfq ? zec(standing.price_zats + standing.deposit_zats) : zec(standing.owed_zats)}</dd></div>
        </dl>
      )}
      {standing.outcome === "reveal_now" && (
        <div className="hairline-t flex flex-col gap-3 pt-5">
          {saved.bid && saved.salt ? (
            <>
              <p className="text-[14px]">Reveal {zec(Number(saved.bid))}{standing.reveal_until ? `, before ${dateTime(standing.reveal_until)}` : ""}.</p>
              <Button onClick={() => reveal.mutate()} disabled={reveal.isPending}>{reveal.isPending ? "Revealing…" : "Reveal my bid"}</Button>
              {reveal.isError && <p className="text-[13px] text-danger">{(reveal.error as Error).message}</p>}
            </>
          ) : (
            <p className="text-[13px] text-muted-foreground">This browser does not hold the amount and reveal code for this bid. Use the device you bid from, or import the bid key and reveal code on the status page.</p>
          )}
        </div>
      )}
      {standing.settlement && (
        <div className="hairline-t flex flex-col gap-4 pt-5">
          <div>
            <Eyebrow>Pay the balance</Eyebrow>
            <p className="mt-1 text-[13px] text-muted-foreground">
              {zec(standing.settlement.amount_zats)} to the auction address, with the memo below so the auction knows it is yours.
              {standing.settle_by ? ` Due by ${dateTime(standing.settle_by)}.` : ""}
            </p>
          </div>
          <div className="flex flex-col items-center gap-5 sm:flex-row sm:items-start">
            <div className="rounded-[2px] bg-qr p-3">
              <QRCodeSVG value={standing.settlement.uri} size={160} level="M" bgColor="transparent" fgColor="#1a1816" aria-label="Settlement QR code" />
            </div>
            <div className="flex min-w-0 flex-1 flex-col gap-3">
              <a href={standing.settlement.uri} className="inline-flex min-h-11 items-center justify-center rounded-[2px] bg-foreground px-5 text-[13px] font-medium tracking-[0.02em] text-background hover:opacity-90">
                Open in wallet
              </a>
              <Button variant="secondary" onClick={() => copy(standing.settlement!.uri)}>
                {copied ? "Copied" : "Copy payment link"}
              </Button>
              <Mono className="block text-[12px] text-muted-foreground">{standing.settlement.memo}</Mono>
            </div>
          </div>
          <div className="hairline-t pt-4">
            <Eyebrow>No ZEC?</Eyebrow>
            {network === "main" ? (
              <div className="mt-3">
                <GoDark
                  amountZats={standing.settlement.amount_zats.toString()}
                  label="Pay the balance from Solana or Ethereum"
                  envelope={{ auctionApi: API_BASE, auctionId, payload: standing.settlement.memo }}
                />
              </div>
            ) : (
              <p className="mt-1 text-[13px] text-muted-foreground">On mainnet, Go Dark pays the balance from a Solana or Ethereum wallet. This instance runs on testnet, where the swap rails do not operate.</p>
            )}
          </div>
        </div>
      )}
      {refund && (
        <div className="hairline-t pt-4 text-[13px] text-muted-foreground">
          <Eyebrow>Refund</Eyebrow>
          <p className="mt-1">
            {refund.status === "sent" && refund.txid ? (
              <>
                Sent, {zec(refund.value_zats)}.{" "}
                <a href={explorerTxUrl(network, refund.txid)} target="_blank" rel="noreferrer" className="num underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">
                  {refund.txid.slice(0, 16)}…
                </a>
              </>
            ) : (
              `${zec(refund.value_zats)} queued; it leaves after a random delay so nothing links it to this lot.`
            )}
          </p>
        </div>
      )}
      {standing.fulfilment && (
        <div className="hairline-t pt-4">
          <Eyebrow>From the seller · only you can see this</Eyebrow>
          <p className="mt-2 whitespace-pre-wrap text-[14px] leading-relaxed">{standing.fulfilment}</p>
        </div>
      )}
      {(standing.outcome === "won" || standing.outcome === "settled") && !standing.fulfilment && (
        <p className="hairline-t pt-4 text-[13px] text-muted-foreground">The seller has not posted delivery instructions yet. They appear here, and only here.</p>
      )}
    </Panel>
  );
}
