"use client";

import { useQuery } from "@tanstack/react-query";
import { verifyInclusion } from "@blind-bazaar/sdk";
import { api, explorerTxUrl } from "@/lib/api";
import { duration, zec } from "@/lib/format";
import { fmt, isRfq } from "@/lib/formats";
import { LotImage } from "@/components/lot-plate";
import { Eyebrow, Fact, LinkButton, Mono, Panel, Skeleton, StatusDot } from "@/components/ui";
import { StandingPanel } from "@/components/standing-panel";

export function ResultClient({ id }: { id: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const lot = useQuery({ queryKey: ["auction", id], queryFn: () => api.auction(id) });
  const { data, isPending, isError, error } = useQuery({
    queryKey: ["result", id],
    queryFn: () => api.result(id),
    refetchInterval: 15_000,
    retry: false,
  });
  const network = health.data?.network ?? "test";
  const priceProof = useQuery({
    queryKey: ["price-proof", id, data?.price_proof?.receipt],
    queryFn: async () => {
      const p = data!.price_proof!;
      const ok = await verifyInclusion(p.receipt, BigInt(p.bid_zats), p.nonce, p.proof);
      return ok && p.proof.root === data!.ledger_hash;
    },
    enabled: !!data?.price_proof && !!data?.ledger_hash,
  });

  if (isPending) {
    return (
      <div className="flex flex-col gap-10">
        <Skeleton className="aspect-[16/7]" />
        <Skeleton className="h-24 w-2/3" />
      </div>
    );
  }
  if (isError) {
    return (
      <div className="flex flex-col items-start gap-4">
        <p className="serif text-[28px]">{error.message === "auction is still open" ? "This lot is still open." : "No result yet."}</p>
        <LinkButton href={`/a/${id}`} variant="secondary">
          Back to the lot
        </LinkButton>
      </div>
    );
  }

  const sent = data.refunds.filter((r) => r.kind === "refund" && r.status === "sent").length;
  const pending = data.refunds.filter((r) => r.kind === "refund" && r.status !== "sent").length;
  const disclosure = (Array.isArray(data.disclosure) ? data.disclosure[0] : data.disclosure) as { verified_against_chain?: boolean; cmx?: string; action_index?: number } | null;
  const revealing = data.status === "revealing";
  const sold = data.winning_amount_zats !== null;
  const uniform = data.format === "uniform";
  const vickrey = data.format === "vickrey";
  const rfq = isRfq(data.format);
  const f = fmt(data.format);
  type D = { verified_against_chain?: boolean; enc_key?: string; bid_zats?: number; reserve_zats?: number };
  const disclosures = Array.isArray(data.disclosure) ? (data.disclosure as D[]) : data.disclosure ? [data.disclosure as D] : [];
  const reserveProofs = disclosures.filter((d) => d.enc_key && d.bid_zats !== undefined && d.reserve_zats !== undefined);
  const reserveMet = reserveProofs.length > 0 && reserveProofs.every((d) => (rfq ? (d.bid_zats ?? 0) <= (d.reserve_zats ?? 0) : (d.bid_zats ?? 0) >= (d.reserve_zats ?? 0)));
  const verifiedCount = disclosures.filter((d) => d.verified_against_chain).length;

  return (
    <div className="flex flex-col gap-10 sm:gap-14">
      <div className="plate rise relative">
        <LotImage id={id} src={lot.data?.image_url} alt={lot.data?.title ?? ""} ratio="16/7" className={sold ? "" : "opacity-60"} />
        <div className="absolute bottom-0 left-0 flex items-end gap-5 p-5 sm:p-8">
          <span className="rounded-[2px] bg-background/90 px-3 py-2 text-[11px] uppercase tracking-[0.14em] text-foreground backdrop-blur">
            {revealing ? "Revealing" : sold ? (rfq ? "Awarded" : "Sold") : rfq ? "No award" : "Unsold"}
          </span>
        </div>
      </div>

      <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
        <div className="flex flex-col gap-10 sm:col-span-7">
          <div className="rise">
            <div className="flex items-center gap-4">
              <Eyebrow>Lot {id} · result</Eyebrow>
              <StatusDot status={data.status} />
            </div>
            {lot.data && <p className="serif mt-4 text-[26px] leading-tight text-muted-foreground">{lot.data.title}</p>}
            <h1 className="serif mt-2 text-[56px] leading-[0.95] sm:text-[96px]">{revealing ? "Not yet." : sold ? zec(data.winning_amount_zats!) : "No sale."}</h1>
            {revealing && data.sealed && (
              <p className="mt-6 max-w-[52ch] text-[17px] leading-[1.6] text-muted-foreground">
                The lot closed with {data.sealed.sealed + data.sealed.revealed} sealed bids and nobody, including the auction, knew any amount. {data.sealed.revealed} revealed so far{data.sealed.reveal_until ? `; the window ends ${duration(Math.max(0, data.sealed.reveal_until - Math.floor(Date.now() / 1000)))} from now` : ""}. Unrevealed bids are refunded in full and cannot win. The commitment root below was fixed before any reveal.
              </p>
            )}
            <p className="dropcap mt-8 max-w-[52ch] text-[17px] leading-[1.6] text-muted-foreground">
              {sold
                ? uniform
                  ? `${data.units_sold} of ${data.units} units sold at one price, the lowest winning bid among ${data.valid_bid_count} sealed bids. Winners above the clearing price pay only the clearing price. Losing amounts and bidders are never revealed. Every losing deposit returns shielded to the address its bidder sealed into the memo.`
                  : vickrey
                    ? data.price_source === "second_bid"
                      ? `The highest of ${data.valid_bid_count} sealed bids won and pays the second-highest bid. The winning bid itself stays sealed, even from the seller; the price is proven from the ledger below. Every losing deposit returns shielded to the address its bidder sealed into the memo.`
                      : `The only bid at or above reserve won and pays the reserve. The winning bid itself stays sealed. Every other deposit returns shielded to the address its bidder sealed into the memo.`
                    : rfq
                      ? `The lowest of ${data.valid_bid_count} sealed quotes within budget. The other quotes and quoters are never revealed, not even to the buyer. Every losing bond returns shielded to the address its quoter sealed into the memo.`
                      : `The highest of ${data.valid_bid_count} sealed bids. The losing amounts and bidders are never revealed, not even to the seller. Every losing deposit returns shielded to the address its bidder sealed into the memo.`
                : rfq
                  ? "No quote came in under budget. Every bond is being returned shielded to the address its quoter sealed into the memo. Nobody learns who quoted, or how much."
                  : "No bid met the reserve. Every deposit is being returned shielded to the address its bidder sealed into the memo. Nobody learns who bid, or how much."}
            </p>
          </div>
          <div className="rise rise-2 grid grid-cols-2 gap-8 sm:grid-cols-3">
            <Fact label={rfq ? "Sealed quotes" : "Sealed bids"} large>
              {data.valid_bid_count}
            </Fact>
            {uniform && (
              <Fact label="Units sold" large>
                {data.units_sold}
                <span className="text-muted-foreground"> / {data.units}</span>
              </Fact>
            )}
            <Fact label={data.refund_latency_secs !== null ? "Refunded within" : "Refunds"} large>
              {data.refund_latency_secs !== null ? duration(data.refund_latency_secs) : sent}
              {data.refund_latency_secs === null && pending > 0 && <span className="text-muted-foreground"> +{pending} pending</span>}
            </Fact>
            <Fact label="Settlement">
              {data.payout_txid
                ? rfq ? "Quoter paid" : "Seller paid"
                : sold
                  ? rfq ? (data.units_settled ? "Buyer paid" : "Awaiting the buyer") : `${data.units_settled} of ${data.units_sold} paid`
                  : "—"}
            </Fact>
          </div>
          <div className="rise rise-3 hairline-t flex flex-col gap-6 pt-8">
            {data.winning_txids.length > 0 && (
              <div>
                <Eyebrow>{data.winning_txids.length > 1 ? "Winning transactions" : "Winning transaction"}</Eyebrow>
                <ul className="mt-2 flex flex-col gap-1.5">
                  {data.winning_txids.map((t) => (
                    <li key={t}>
                      <a
                        href={explorerTxUrl(network, t)}
                        target="_blank"
                        rel="noreferrer"
                        className="num block break-all text-[13px] underline decoration-hairline-strong underline-offset-4 hover:decoration-accent"
                      >
                        {t}
                      </a>
                    </li>
                  ))}
                </ul>
                <p className="mt-1 text-[13px] text-muted-foreground">Indistinguishable on the explorer from any other shielded transaction. Only the proof ties them to this lot.</p>
              </div>
            )}
            {data.sealed?.commit_hash && (
              <div>
                <Eyebrow>Commitment root at close</Eyebrow>
                <Mono className="mt-2 block text-muted-foreground">{data.sealed.commit_hash}</Mono>
                <p className="mt-1 text-[13px] text-muted-foreground">A hash of every sealed commitment the moment the lot closed, before any amount was known. Reveals that do not match a committed amount are rejected.</p>
              </div>
            )}
            <div>
              <Eyebrow>Ledger commitment at close</Eyebrow>
              <Mono className="mt-2 block text-muted-foreground">{data.ledger_hash ?? "—"}</Mono>
              <p className="mt-1 text-[13px] text-muted-foreground">
                A hash of every valid {f.verb} at the moment of close, published before any refund leaves.
                {lot.data?.close_height ? ` Close is fixed by the chain: bids mined after block ${lot.data.close_height.toLocaleString()} were late.` : ""}
              </p>
            </div>
            {vickrey && sold && (
              <div>
                <Eyebrow>Price proof</Eyebrow>
                {data.price_proof ? (
                  <>
                    <p className="serif mt-2 text-[22px] leading-tight">
                      {priceProof.data === true ? "The second bid is in the ledger." : priceProof.data === false ? "Proof does not verify." : "Checking…"}
                    </p>
                    <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
                      The runner-up&apos;s ledger leaf, receipt <span className="num">{data.price_proof.receipt.slice(0, 12)}…</span> at{" "}
                      <span className="num">{zec(data.price_proof.bid_zats)}</span>, with its Merkle path to the commitment above, verified in this browser. It shows the price the winner pays was a bid sealed before close. The runner-up stays anonymous and the winning bid stays sealed.
                    </p>
                  </>
                ) : (
                  <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">Only one bid met the reserve, so the winner pays the published reserve. Nothing else to prove.</p>
                )}
              </div>
            )}
          </div>
        </div>
        <aside className="rise rise-2 flex flex-col gap-6 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
          <StandingPanel auctionId={id} network={network} />
          <Panel className="flex flex-col gap-5">
            <div>
              <Eyebrow>Proof</Eyebrow>
              <p className="serif mt-2 text-[32px] leading-tight">
                {disclosures.length > 0
                  ? verifiedCount === disclosures.length
                    ? disclosures.length > 1
                      ? `All ${disclosures.length} verified on chain.`
                      : "Opening verified by the service."
                    : `${verifiedCount} of ${disclosures.length} verified.`
                  : sold
                    ? "Pending."
                    : "Nothing to prove."}
              </p>
            </div>
            <p className="text-[14px] leading-relaxed text-foreground">
              In plain words: the auction opens the winning deposit so anyone can confirm that payment exists on the
              Zcash chain, and reveals nothing about any other bidder.{reserveProofs.length > 0 ? " Because the memo key is disclosed, the winner's bid and refund address are readable from that one note." : ""}
            </p>
            <p className="text-[14px] leading-relaxed text-muted-foreground">
              Technically, a note-opening disclosure of the winning output: the recipient, value and randomness that reproduce the
              on-chain commitment. Anyone can check it with the service binary and a public lightwalletd. {rfq ? "Losing quotes" : "Losing bids"}{" "}
              stay sealed{vickrey ? ", and so does the winning bid" : ""}.
            </p>
            {reserveProofs.length > 0 && (
              <div className="hairline-t pt-4">
                <Eyebrow>{rfq ? "Budget" : "Reserve"}</Eyebrow>
                <p className="serif mt-2 text-[22px] leading-tight">{reserveMet ? (rfq ? "Within budget, and provable." : "Met, and provable.") : rfq ? "Over budget." : "Not met."}</p>
                <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
                  The winning note&apos;s encryption key is disclosed, so anyone can decrypt that one memo from the
                  transaction and read the {f.verb}{reserveProofs.length === 1 && reserveProofs[0].bid_zats !== undefined ? ` (${zec(reserveProofs[0].bid_zats)} against a ${rfq ? "budget" : "reserve"} of ${zec(reserveProofs[0].reserve_zats ?? 0)})` : ""}. No other note can be opened with it.
                </p>
              </div>
            )}
            {disclosures.length > 0 && (
              <div className="hairline-t pt-4">
                <Eyebrow>{disclosures.length > 1 ? `${disclosures.length} note openings` : `Commitment · action ${disclosure?.action_index}`}</Eyebrow>
                {disclosures.length === 1 && <Mono className="mt-2 block text-muted-foreground">{disclosure?.cmx}</Mono>}
                <details className="mt-4 text-[13px]">
                  <summary className="cursor-pointer text-muted-foreground hover:text-foreground">Full disclosure JSON</summary>
                  <pre className="num mt-3 overflow-x-auto text-[11px] leading-relaxed text-muted-foreground">{JSON.stringify(data.disclosure, null, 2)}</pre>
                </details>
              </div>
            )}
            <div className="flex flex-wrap gap-3">
              {disclosures.length > 0 && (
                <LinkButton href={`/a/${id}/verify`}>Re-check in my browser</LinkButton>
              )}
              <LinkButton href={`/a/${id}`} variant="secondary">
                Back to the lot
              </LinkButton>
            </div>
          </Panel>
        </aside>
      </div>
    </div>
  );
}
