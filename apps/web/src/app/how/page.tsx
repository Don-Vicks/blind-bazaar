import Link from "next/link";
import { Eyebrow } from "@/components/ui";
import { Glossary } from "@/components/explain";

const steps = [
  ["You bid", "On the lot page you type your bid and the address your refund should go to. The site turns that into a payment request for the deposit. You pay it from your Zcash wallet. Your bid travels inside that payment, encrypted, where only the auction can read it."],
  ["The auction confirms it arrived", "The auction reads the payment, checks the deposit is right, and publishes a receipt. A receipt is a code that proves your bid landed without saying who you are or what you bid. You keep it."],
  ["Bidding closes", "The highest bid wins (details per format below). Before any money moves, the auction publishes a fingerprint of every bid. That fingerprint cannot be changed later, so bids cannot be added, removed or altered after the fact."],
  ["Losers are refunded", "Every losing deposit goes back to the address its bidder gave, as a private payment, after a random delay. Nothing on the chain links the refund to the lot."],
  ["The winner pays", "The winner pays the rest of the price, with a small code in the payment so the auction knows it is theirs. Then the seller is paid, privately."],
  ["Anyone can check", "The auction opens the winning payment: enough detail to confirm it exists on the Zcash chain for the stated amount, and nothing more. Losing bids stay sealed forever. This site re-checks the proof in your browser."],
];

const formats = [
  ["Sealed first price", "The highest bid at or above the reserve wins and pays its own bid.", "The classic sealed auction."],
  ["Vickrey (second price)", "The highest bid wins but pays the second-highest bid, or the reserve if there is none. The winning bid is never revealed. The price is proven by showing the runner-up's entry in the bid fingerprint.", "Bid what the item is worth to you; there is no advantage in shading your bid."],
  ["Edition (uniform price)", "Several identical units. The top N bids win and every winner pays the same price, the lowest winning bid.", "Prints, tickets, tokens."],
  ["Request for quotes", "The roles flip: the lister is a buyer with a budget. Suppliers post a bond and seal a quote. The lowest quote within budget wins; the buyer pays it; the winner gets the quote plus the bond back.", "Private procurement: nobody sees the other quotes."],
  ["Sealed-reveal (option on any of the above)", "Bids carry a commitment, not the amount, so not even the auction knows a bid before close. After close bidders reveal within a window; whoever does not is refunded in full and cannot win.", "When bidders do not want to trust the operator with their bid while the lot is open."],
];

const parties = [
  ["Other bidders", "That a lot exists, how many bids arrived, its result, and the list of receipts.", "Any losing bid amount, or who bid. (Vickrey publishes the second-highest amount, not who placed it.)"],
  ["Someone watching the blockchain", "Private transactions of unknown amount and origin.", "Which transactions belong to the lot."],
  ["The seller", "The winning price and the payout.", "Losing bids and losing bidders."],
  ["The auction operator", "On a classic lot, the bids while it is open (it holds the read-only key). On a sealed-reveal lot, only that deposits arrived.", "Under the escrow setup it cannot move the money alone: payouts need separate signers, each of which inspects the payment first. (A single-key instance, used for development, can.)"],
  ["The swap service (Go Dark)", "Where your Solana or Ethereum funds came from, and the Zcash address paid.", "The bid, which never passes through it."],
];

export default function HowPage() {
  return (
    <div className="flex flex-col gap-16 sm:gap-24">
      <section className="grid gap-10 sm:grid-cols-12 sm:gap-10">
        <div className="sm:col-span-7">
          <Eyebrow className="rise">How it works</Eyebrow>
          <h1 className="serif rise rise-2 mt-4 text-[48px] leading-[0.98] sm:text-[72px]">
            A bid is a private payment.
            <br />
            <span className="italic text-muted-foreground">The blockchain is the mailbox.</span>
          </h1>
          <p className="rise rise-3 mt-8 max-w-[48ch] text-[17px] leading-[1.6] text-muted-foreground">
            There is no smart contract and no account. Every Zcash payment can carry a short encrypted note. A bidder
            pays the deposit and writes the bid in that note. The auction holds a read-only key that can decrypt the
            notes but cannot spend anything. At close it publishes the result and a proof. That is the whole system.
          </p>
        </div>
        <div className="rise rise-3 sm:col-span-5">
          <Diagram />
        </div>
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">Six steps, start to finish</h2>
        </div>
        <ol className="grid gap-x-10 gap-y-8 sm:grid-cols-2">
          {steps.map(([title, body], i) => (
            <li key={title} className="flex gap-5">
              <span className="serif shrink-0 text-[40px] leading-none text-accent">{i + 1}</span>
              <div>
                <p className="serif text-[22px] leading-tight">{title}</p>
                <p className="mt-2 text-[14px] leading-relaxed text-muted-foreground">{body}</p>
              </div>
            </li>
          ))}
        </ol>
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">Four formats</h2>
          <p className="mt-1 text-[13px] text-muted-foreground">The seller picks one when listing. Everything else works the same way.</p>
        </div>
        <div className="grid gap-px overflow-hidden rounded-md border border-hairline bg-hairline sm:grid-cols-3">
          <div className="hidden bg-background p-4 sm:block"><Eyebrow>Format</Eyebrow></div>
          <div className="hidden bg-background p-4 sm:block"><Eyebrow>Rule</Eyebrow></div>
          <div className="hidden bg-background p-4 sm:block"><Eyebrow>Good for</Eyebrow></div>
          {formats.map(([name, rule, good]) => (
            <div key={name} className="contents">
              <div className="bg-background p-4"><p className="serif text-[20px]">{name}</p></div>
              <div className="bg-background p-4 text-[14px] text-muted-foreground">{rule}</div>
              <div className="bg-background p-4 text-[14px] text-muted-foreground">{good}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">Who sees what</h2>
        </div>
        <div className="grid gap-px overflow-hidden rounded-md border border-hairline bg-hairline sm:grid-cols-3">
          <div className="hidden bg-background p-4 sm:block"><Eyebrow>Party</Eyebrow></div>
          <div className="hidden bg-background p-4 sm:block"><Eyebrow>Sees</Eyebrow></div>
          <div className="hidden bg-background p-4 sm:block"><Eyebrow>Never sees</Eyebrow></div>
          {parties.map(([who, sees, never]) => (
            <div key={who} className="contents">
              <div className="bg-background p-4"><p className="serif text-[20px]">{who}</p></div>
              <div className="bg-background p-4 text-[14px] text-muted-foreground"><span className="sm:hidden eyebrow block mb-1">Sees</span>{sees}</div>
              <div className="bg-background p-4 text-[14px] text-muted-foreground"><span className="sm:hidden eyebrow block mb-1">Never sees</span>{never}</div>
            </div>
          ))}
        </div>
        <p className="max-w-[60ch] text-[14px] leading-relaxed text-muted-foreground">
          The honest limit on classic lots: the operator reads bids while a lot is open (sealed-reveal lots remove
          this; the seller chooses). What stops it from cheating is that it
          cannot alter the published fingerprint and cannot forge the proof. With the escrow setup it also cannot move
          money alone: the funds sit under a key split three ways, and two separate signers must inspect and approve
          every payout (built and unit-tested; the first on-chain run is pending). Hiding bids
          from the operator too is the next step on the{" "}
          <Link href="https://github.com/Don-Vicks/blind-bazaar/blob/main/ROADMAP.md" className="text-foreground underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">
            roadmap
          </Link>
          .
        </p>
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">What you can check yourself</h2>
        </div>
        <ul className="grid gap-6 text-[14px] leading-relaxed text-muted-foreground sm:grid-cols-3">
          <li><span className="serif block text-[20px] text-foreground">Your bid was counted</span>On the lot&apos;s status page, enter your receipt. After close, your browser checks your bid sits inside the published fingerprint.</li>
          <li><span className="serif block text-[20px] text-foreground">The winning payment is real</span>The verify page rebuilds the winning payment&apos;s commitment with the same code the Zcash chain uses; the command-line verifier fetches the transaction from a public Zcash node and compares it there.</li>
          <li><span className="serif block text-[20px] text-foreground">The price is honest</span>The reserve is proven from the winning bid&apos;s decrypted note; in a Vickrey lot the second price is proven from the runner-up&apos;s fingerprint entry.</li>
        </ul>
      </section>

      <Glossary />
    </div>
  );
}

function Diagram() {
  const c = "var(--foreground)";
  const m = "var(--muted-foreground)";
  const a = "var(--accent)";
  return (
    <svg viewBox="0 0 420 520" className="w-full" role="img" aria-label="How it fits together: bidders and Go Dark pay into a fresh Zcash address per lot; the auction reads bids with a read-only key; two of three payout signers approve refunds and the seller payout; the result page publishes receipts, the bid fingerprint and the proof.">
      <defs>
        <marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M0 0L10 5L0 10z" fill={m} />
        </marker>
      </defs>
      <g fontFamily="var(--font-sans)" fontSize="12" fill={c}>
        <rect x="10" y="20" width="180" height="56" rx="3" fill="none" stroke={m} />
        <text x="24" y="44">Bidder wallet</text>
        <text x="24" y="62" fill={m}>Zodl · Zingo · YWallet</text>
        <rect x="230" y="20" width="180" height="56" rx="3" fill="none" stroke={m} />
        <text x="244" y="44">Go Dark</text>
        <text x="244" y="62" fill={m}>pay from Solana or Ethereum</text>

        <line x1="100" y1="76" x2="190" y2="140" stroke={m} markerEnd="url(#arr)" />
        <line x1="320" y1="76" x2="230" y2="140" stroke={m} markerEnd="url(#arr)" />
        <text x="18" y="118" fill={m} fontSize="11">deposit, bid inside the note</text>
        <text x="262" y="118" fill={m} fontSize="11">swapped to ZEC, bid relayed</text>

        <rect x="80" y="140" width="260" height="60" rx="3" fill={a} fillOpacity="0.08" stroke={a} />
        <text x="96" y="164">Zcash, private payments</text>
        <text x="96" y="182" fill={m}>a fresh address for every lot</text>

        <line x1="210" y1="200" x2="210" y2="250" stroke={m} markerEnd="url(#arr)" />
        <text x="222" y="230" fill={m} fontSize="11">read-only key</text>

        <rect x="10" y="250" width="190" height="72" rx="3" fill="none" stroke={m} />
        <text x="24" y="274">Auction (reads only)</text>
        <text x="24" y="292" fill={m}>reads bids, keeps the clock,</text>
        <text x="24" y="308" fill={m}>publishes receipts and results</text>

        <rect x="220" y="250" width="190" height="72" rx="3" fill="none" stroke={m} />
        <text x="234" y="274">Payout signers (2 of 3)</text>
        <text x="234" y="292" fill={m}>check every payment, then</text>
        <text x="234" y="308" fill={m}>approve refunds and payouts</text>

        <line x1="200" y1="286" x2="220" y2="286" stroke={m} strokeDasharray="3 3" />
        <text x="130" y="344" fill={m} fontSize="11">no single server can move the money</text>

        <line x1="315" y1="322" x2="315" y2="370" stroke={m} markerEnd="url(#arr)" />
        <text x="326" y="350" fill={m} fontSize="11">private refunds · seller payout</text>

        <rect x="80" y="370" width="260" height="60" rx="3" fill={a} fillOpacity="0.08" stroke={a} />
        <text x="96" y="394">Zcash, private payments</text>
        <text x="96" y="412" fill={m}>losers refunded · seller paid</text>

        <line x1="105" y1="322" x2="105" y2="450" stroke={m} markerEnd="url(#arr)" />
        <rect x="10" y="450" width="400" height="56" rx="3" fill="none" stroke={m} />
        <text x="24" y="474">Result page</text>
        <text x="24" y="492" fill={m}>receipts · bid fingerprint · proof of the winning payment</text>
      </g>
    </svg>
  );
}
