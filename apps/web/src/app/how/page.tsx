import Link from "next/link";
import { Eyebrow } from "@/components/ui";

const steps = [
  ["Seal", "Your bid is written into the encrypted memo of one shielded payment of the fixed deposit. The chain is the mailbox; only the auction's viewing key can open it."],
  ["Receive", "The service decrypts the memo, validates the deposit and nonce, and publishes a receipt, a hash of your nonce, so you can confirm it landed without revealing anything."],
  ["Close", "The top bid wins, or for editions the top N win and all pay the lowest winning bid. A hash of the full ledger is published before any money moves."],
  ["Refund", "Every losing deposit returns as a shielded payment to the address its bidder sealed into the memo, after a random delay. Nothing links refunds to the lot."],
  ["Settle", "Winners pay the balance with a settlement memo. The seller receives the price, shielded."],
  ["Prove", "The winning outputs are opened: recipient, value and randomness that reproduce the on-chain commitment, so anyone can check the result. Losing bids stay sealed forever."],
];

const parties = [
  ["Other bidders", "That a lot exists, its result, the receipt list.", "Any bid, any bidder, how many bids."],
  ["Chain observer", "Shielded transactions of unknown amount and origin.", "Which transactions belong to the lot."],
  ["Seller", "The price and the payout.", "Losing bids and losing bidders."],
  ["Viewer process", "Decrypted memos (holds the viewing key).", "It cannot spend: refunds and payouts are signed by a separate spender that recomputes the result from its own view."],
  ["Swap rail (Go Dark)", "Your source address and the Zcash destination.", "The bid, which never travels through the rail."],
];

export default function HowPage() {
  return (
    <div className="flex flex-col gap-16 sm:gap-24">
      <section className="grid gap-10 sm:grid-cols-12 sm:gap-10">
        <div className="sm:col-span-7">
          <Eyebrow className="rise">How it works</Eyebrow>
          <h1 className="serif rise rise-2 mt-4 text-[48px] leading-[0.98] sm:text-[72px]">
            No contract.
            <br />
            <span className="italic text-muted-foreground">The chain is the mailbox.</span>
          </h1>
          <p className="dropcap rise rise-3 mt-10 max-w-[46ch] text-[17px] leading-[1.6] text-muted-foreground">
            Zcash already has what a sealed-bid auction needs: a 512-byte encrypted memo on every shielded payment, a
            viewing key that reads without spending, and a note commitment that can be opened later to prove one
            output and only that output. Blind Bazaar is those three primitives and a small service that keeps time.
          </p>
        </div>
        <div className="rise rise-3 sm:col-span-5">
          <Diagram />
        </div>
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">Six steps</h2>
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
          The known limit, stated plainly: whoever holds the viewing key reads bids while a lot is open. The
          viewer/spender split stops that party from moving funds; threshold signing and MPC bid selection are the
          next steps on the{" "}
          <Link href="https://github.com/Don-Vicks/blind-bazaar/blob/main/ROADMAP.md" className="text-foreground underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">
            roadmap
          </Link>
          .
        </p>
      </section>
    </div>
  );
}

function Diagram() {
  const c = "var(--foreground)";
  const m = "var(--muted-foreground)";
  const a = "var(--accent)";
  return (
    <svg viewBox="0 0 420 520" className="w-full" role="img" aria-label="Architecture: bidders and Go Dark pay shielded ZEC into a per-lot address; the viewer reads memos with the viewing key; the spender signs refunds and payouts; the result page publishes receipts, the ledger hash and the proof.">
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
        <text x="244" y="62" fill={m}>Phantom · MetaMask → rail</text>

        <line x1="100" y1="76" x2="190" y2="140" stroke={m} markerEnd="url(#arr)" />
        <line x1="320" y1="76" x2="230" y2="140" stroke={m} markerEnd="url(#arr)" />
        <text x="18" y="118" fill={m} fontSize="11">ZIP-321 · bid in memo</text>
        <text x="262" y="118" fill={m} fontSize="11">shielded ZEC + envelope</text>

        <rect x="80" y="140" width="260" height="60" rx="3" fill={a} fillOpacity="0.08" stroke={a} />
        <text x="96" y="164">Zcash shielded pool</text>
        <text x="96" y="182" fill={m}>one fresh unified address per lot</text>

        <line x1="210" y1="200" x2="210" y2="250" stroke={m} markerEnd="url(#arr)" />
        <text x="222" y="230" fill={m} fontSize="11">viewing key</text>

        <rect x="10" y="250" width="190" height="72" rx="3" fill="none" stroke={m} />
        <text x="24" y="274">Viewer</text>
        <text x="24" y="292" fill={m}>decrypts memos, runs the clock,</text>
        <text x="24" y="308" fill={m}>publishes receipts and results</text>

        <rect x="220" y="250" width="190" height="72" rx="3" fill="none" stroke={m} />
        <text x="234" y="274">Spender</text>
        <text x="234" y="292" fill={m}>recomputes the outcome itself,</text>
        <text x="234" y="308" fill={m}>signs refunds and payouts</text>

        <line x1="200" y1="286" x2="220" y2="286" stroke={m} strokeDasharray="3 3" />
        <text x="130" y="344" fill={m} fontSize="11">lot definitions only, pinned at first sight</text>

        <line x1="315" y1="322" x2="315" y2="370" stroke={m} markerEnd="url(#arr)" />
        <text x="326" y="350" fill={m} fontSize="11">shielded refunds · seller payout</text>

        <rect x="80" y="370" width="260" height="60" rx="3" fill={a} fillOpacity="0.08" stroke={a} />
        <text x="96" y="394">Zcash shielded pool</text>
        <text x="96" y="412" fill={m}>losers refunded · seller paid</text>

        <line x1="105" y1="322" x2="105" y2="450" stroke={m} markerEnd="url(#arr)" />
        <rect x="10" y="450" width="400" height="56" rx="3" fill="none" stroke={m} />
        <text x="24" y="474">Result page</text>
        <text x="24" y="492" fill={m}>receipts · ledger hash · note-opening proof of the winning outputs</text>
      </g>
    </svg>
  );
}
