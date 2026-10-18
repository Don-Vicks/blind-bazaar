"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { api } from "@/lib/api";
import { LinkButton } from "@/components/ui";

/** Plain-language definitions used across the site. Keep each under 25 words. */
export const GLOSSARY: Record<string, string> = {
  Reserve: "The lowest price the seller will accept. Bids below it cannot win. Published on the lot.",
  Deposit: "A fixed amount every bidder pays to enter. Losers get it back. For the winner it counts towards the price.",
  Bid: "The amount you offer. It is hidden inside your deposit payment and nobody can read it until the auction closes.",
  Receipt: "A code you get when you bid. It proves your bid arrived, without revealing who you are or what you bid.",
  "Ledger commitment": "A fingerprint of every bid at the moment of close, published before any money moves. It cannot be changed afterwards.",
  Proof: "Data that lets anyone check the winning payment really exists on the Zcash chain. Losing bids are never part of it.",
  Settlement: "After winning, you pay the rest of the price (your bid minus the deposit). Then the seller is paid.",
  Budget: "In a request for quotes, the most the buyer will pay. Quotes above it cannot win.",
  Bond: "In a request for quotes, what each supplier posts to quote. Returned when they lose, or with the payout when they win.",
  "Unified address": "A Zcash address starting with u1 (or utest1 on the test network). Payments to it are private.",
};

/** A word with a plain-language definition on hover and focus. */
export function Term({ children, of }: { children: React.ReactNode; of: keyof typeof GLOSSARY }) {
  return (
    <abbr title={GLOSSARY[of]} className="cursor-help underline decoration-dotted decoration-hairline-strong underline-offset-4 no-underline-skip">
      {children}
    </abbr>
  );
}

/** Which Zcash network this instance runs on, in words a visitor understands. */
export function NetworkNotice({ compact = false }: { compact?: boolean }) {
  const { data } = useQuery({ queryKey: ["health"], queryFn: api.health, refetchInterval: 30_000 });
  if (!data) return null;
  const test = data.network !== "main";
  const custody = data.signing?.startsWith("frost")
    ? "Funds are held by a 2-of-3 key split across separate signers; no single server can move them."
    : data.signing === "seed"
      ? "Funds are held by a single operator key on this instance."
      : null;
  return (
    <div className={`rounded-md border border-hairline bg-surface ${compact ? "px-4 py-3 text-[13px]" : "px-5 py-4 text-[14px]"} text-muted-foreground`}>
      <span className="text-foreground">{test ? "Test network." : "Mainnet."}</span>{" "}
      {test
        ? "This instance runs on Zcash testnet. Bids use test ZEC, which has no value. Get some free at a testnet faucet and try a bid."
        : "Bids here use real ZEC."}
      {custody && !compact ? ` ${custody}` : null}
    </div>
  );
}

/** The three ways in: bid, sell, check. Used on the home page. */
export function StartHere() {
  const cards = [
    {
      title: "I want to bid",
      lines: [
        "Open a lot and type your bid and a refund address.",
        "Scan the QR with a Zcash wallet and pay the deposit. Your bid rides inside that payment, encrypted.",
        "Keep your receipt. If you lose, the deposit comes back to you privately.",
      ],
      href: "/#open",
      cta: "See open lots",
    },
    {
      title: "I want to sell",
      lines: [
        "Sign in with a seller key and list a lot: title, reserve, deposit, how long it runs.",
        "Bidders pay to a fresh address created for your lot. You see how many bids, never the amounts.",
        "At close you learn the winning price. Once the winner pays, you are paid, privately.",
      ],
      href: "/dashboard",
      cta: "Open the seller dashboard",
    },
    {
      title: "I want to check a result",
      lines: [
        "Every closed lot has a result page with the winning price.",
        "It publishes a fingerprint of all bids and a proof of the winning payment on the Zcash chain.",
        "Your browser re-derives the proof, and a one-line command checks it against the chain itself.",
      ],
      href: "/#closed",
      cta: "See closed lots",
    },
  ];
  return (
    <section aria-label="Start here" className="grid gap-6 sm:grid-cols-3">
      {cards.map((c) => (
        <div key={c.title} className="flex flex-col gap-4 rounded-md border border-hairline bg-surface p-6">
          <p className="serif text-[24px] leading-tight">{c.title}</p>
          <ol className="flex flex-col gap-2 text-[14px] leading-relaxed text-muted-foreground">
            {c.lines.map((l, i) => (
              <li key={i} className="flex gap-3">
                <span className="num shrink-0 text-accent">{i + 1}</span>
                <span>{l}</span>
              </li>
            ))}
          </ol>
          <LinkButton href={c.href} variant="secondary" className="mt-auto self-start">
            {c.cta}
          </LinkButton>
        </div>
      ))}
    </section>
  );
}

/** What each word on this site means. */
export function Glossary({ only }: { only?: (keyof typeof GLOSSARY)[] }) {
  const keys = only ?? (Object.keys(GLOSSARY) as (keyof typeof GLOSSARY)[]);
  return (
    <section aria-labelledby="glossary-heading" className="flex flex-col gap-6">
      <div className="hairline-b pb-4">
        <h2 id="glossary-heading" className="serif text-[32px]">
          What the words mean
        </h2>
      </div>
      <dl className="grid gap-x-10 gap-y-5 sm:grid-cols-2">
        {keys.map((k) => (
          <div key={k}>
            <dt className="serif text-[20px]">{k}</dt>
            <dd className="mt-1 text-[14px] leading-relaxed text-muted-foreground">{GLOSSARY[k]}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

/** Three reasons to trust the result, in plain words, with the honest caveat. */
export function WhyTrust() {
  const { data } = useQuery({ queryKey: ["health"], queryFn: api.health });
  const frost = data?.signing?.startsWith("frost") ?? false;
  const items = [
    ["Bids are encrypted on the blockchain", "A bid is a private Zcash payment. Nobody can read it except the auction, which has a read-only key. Other bidders cannot even tell you bid."],
    ["Results are provable", "At close the auction publishes a fingerprint of every bid, then a proof of the winning payment. Anyone can check both; this site does it in your browser."],
    [
      frost ? "No single server holds the money" : "The money is held by the operator",
      frost
        ? "Refunds and payouts need two of three separate signers to approve each payment after inspecting it. A hacked server cannot run off with the pool."
        : "On this instance one operator key signs refunds and payouts. The production setup splits that key across three signers so no single server can move funds.",
    ],
  ];
  return (
    <section aria-labelledby="trust-heading" className="flex flex-col gap-6">
      <div className="hairline-b flex items-baseline justify-between pb-4">
        <h2 id="trust-heading" className="serif text-[32px]">
          Why you can trust the result
        </h2>
        <Link href="/how" className="text-[13px] text-muted-foreground underline decoration-hairline-strong underline-offset-4 hover:text-foreground">
          Full explanation
        </Link>
      </div>
      <div className="grid gap-8 sm:grid-cols-3">
        {items.map(([t, b]) => (
          <div key={t}>
            <p className="serif text-[22px] leading-tight">{t}</p>
            <p className="mt-2 text-[14px] leading-relaxed text-muted-foreground">{b}</p>
          </div>
        ))}
      </div>
      <p className="text-[13px] text-muted-foreground">
        The honest limit: on a classic lot the auction operator can read bids while it is open (sealed-reveal lots hide them from the operator too). It cannot change them or hide them from the fingerprint{frost ? ", and it cannot spend the money alone" : "; on this instance it also holds the spending key"}.
      </p>
    </section>
  );
}
