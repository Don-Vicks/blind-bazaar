import Link from "next/link";
import { Eyebrow, LinkButton } from "@/components/ui";
import { LotPlate } from "@/components/lot-plate";
import { PayButtonBuilder } from "@/components/pay-button-builder";

export default function EmbedPage() {
  const site = process.env.NEXT_PUBLIC_SITE_URL ?? "";
  return (
    <div className="flex flex-col gap-16 sm:gap-24">
      <section className="grid gap-10 sm:grid-cols-12 sm:gap-10">
        <div className="sm:col-span-7 min-w-0">
          <Eyebrow className="rise">Pay button · Go Dark</Eyebrow>
          <h1 className="serif rise rise-2 mt-4 text-[48px] leading-[0.98] sm:text-[80px]">
            A private pay button
            <br />
            <span className="italic text-muted-foreground">for any site.</span>
          </h1>
          <p className="rise rise-3 mt-8 max-w-[48ch] text-[17px] leading-[1.6] text-muted-foreground">
            Paste one snippet and your page gets a button. A visitor pays from Phantom or MetaMask, in SOL, USDC or
            ETH. You receive shielded Zcash at a private address. Nothing is sent to your server, and on the Zcash
            side nobody can tell from the blockchain that they paid you. No account, no Zcash wallet needed to pay.
          </p>
          <div className="rise rise-4 mt-8 flex flex-wrap gap-3">
            <LinkButton href="#build">Build your button</LinkButton>
            <LinkButton href="/demo/tip-jar.html" variant="secondary">See it on a demo site</LinkButton>
          </div>
        </div>
        <div className="rise rise-3 sm:col-span-5 min-w-0">
          <div className="plate">
            <LotPlate id="go-dark-cover" ratio="4/5" className="block w-full" />
          </div>
          <Eyebrow className="mt-4">Tips, checkout, donations, auction deposits. Same button.</Eyebrow>
        </div>
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">What happens when someone clicks</h2>
        </div>
        <ol className="grid gap-x-10 gap-y-8 sm:grid-cols-4">
          {[
            ["Pick a wallet", "Solana (Phantom) or Ethereum (MetaMask), and the asset to pay with."],
            ["See the quote", "What they send, what you receive, how long it takes, and exactly who sees what. Nothing moves until they confirm."],
            ["Pay", "One transfer from their wallet to a swap service. Ethereum goes through Maya Protocol, Solana through NEAR Intents."],
            ["You receive shielded ZEC", "The swap delivers to your private Zcash address. The button shows the Zcash transaction once it lands."],
          ].map(([t, b], i) => (
            <li key={t} className="flex gap-4">
              <span className="serif shrink-0 text-[36px] leading-none text-accent">{i + 1}</span>
              <div>
                <p className="serif text-[20px] leading-tight">{t}</p>
                <p className="mt-2 text-[14px] leading-relaxed text-muted-foreground">{b}</p>
              </div>
            </li>
          ))}
        </ol>
      </section>

      <section id="build" className="flex flex-col gap-8 scroll-mt-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">Build your button</h2>
          <p className="mt-1 text-[13px] text-muted-foreground">Change anything on the left; the preview and the snippet update.</p>
        </div>
        <PayButtonBuilder site={site} />
      </section>

      <section className="flex flex-col gap-8">
        <div className="hairline-b pb-4">
          <h2 className="serif text-[32px]">Who sees what, honestly</h2>
        </div>
        <dl className="grid gap-6 text-[14px] sm:grid-cols-2">
          {[
            ["Your page", "The wallet the payer connects, because the button runs inside your page. The quote proxy logs no addresses; nothing is sent to your server."],
            ["The payer's chain (Solana or Ethereum)", "A public transfer from their wallet to the swap service. If their wallet is known to be theirs, that transfer is visible. The button tells them so."],
            ["The swap service", "The payer's source address, the amount, and the Zcash address it delivers to. Never a memo. On Ethereum the destination also sits in public calldata, so a fixed address links payments to you; the fresh-address mode avoids that. This is the limit of the privacy: it is private on the Zcash side, not from the rail."],
            ["Zcash", "One shielded payment of unknown amount and recipient. Each payment can go to a fresh address, so two payments to you cannot be linked."],
          ].map(([k, v]) => (
            <div key={k}>
              <dt className="serif text-[20px]">{k}</dt>
              <dd className="mt-1 leading-relaxed text-muted-foreground">{v}</dd>
            </div>
          ))}
        </dl>
        <p className="max-w-[60ch] text-[13px] text-muted-foreground">
          Full threat model, every hop and every mitigation, in{" "}
          <Link href="https://github.com/Don-Vicks/blind-bazaar/blob/main/docs/GO-DARK-THREAT-MODEL.md" className="text-foreground underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">
            the repository
          </Link>
          . The same button, with a sealed bid attached, is how someone without ZEC enters an auction here.
        </p>
      </section>
    </div>
  );
}
