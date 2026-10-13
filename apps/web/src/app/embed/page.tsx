import { Eyebrow, Panel } from "@/components/ui";
import { GoDarkDemo } from "@/components/go-dark-demo";
import { LotPlate } from "@/components/lot-plate";

export default function EmbedPage() {
  const site = process.env.NEXT_PUBLIC_SITE_URL ?? "https://blindbazaar.example";
  const snippet = `<script src="${site}/go-dark.iife.js"></script>
<div id="pay"></div>
<script>
  GoDark.mount(document.getElementById("pay"), {
    to: "u1…",             // your unified address
    amountZats: "1000000", // 0.01 ZEC, delivered shielded
    apiBase: "${site}",
  });
</script>`;
  return (
    <div className="flex flex-col gap-16 sm:gap-24">
      <section className="grid gap-10 sm:grid-cols-12 sm:gap-10">
        <div className="sm:col-span-7">
          <Eyebrow className="rise">Go Dark</Eyebrow>
          <h1 className="serif rise rise-2 mt-4 text-[48px] leading-[0.98] sm:text-[84px]">
            One button.
            <br />
            <span className="italic text-muted-foreground">Shielded ZEC from any wallet.</span>
          </h1>
          <p className="dropcap rise rise-3 mt-10 max-w-[46ch] text-[17px] leading-[1.6] text-muted-foreground">
            A visitor pays from Phantom or MetaMask. The recipient gets shielded ZEC at a unified address. Ethereum
            routes through Maya Protocol, Solana through NEAR Intents. No account, no Zcash wallet needed to pay, and
            the page that hosts the button never sees a memo or logs an address.
          </p>
        </div>
        <div className="rise rise-3 sm:col-span-5">
          <div className="plate">
            <LotPlate id="go-dark-cover" ratio="4/5" className="block w-full" />
          </div>
          <Eyebrow className="mt-4">Every site that adds the button onboards its own visitors into shielded Zcash.</Eyebrow>
        </div>
      </section>

      <section className="grid gap-12 sm:grid-cols-12 sm:gap-10">
        <div className="flex flex-col gap-10 sm:col-span-7">
          <div className="rise hairline-t pt-8">
            <Eyebrow>Embed</Eyebrow>
            <pre className="num mt-4 overflow-x-auto text-[12px] leading-relaxed text-muted-foreground">{snippet}</pre>
            <p className="mt-4 text-[13px] text-muted-foreground">
              React: <span className="num text-foreground">import {"{ GoDark }"} from &quot;@blind-bazaar/go-dark&quot;</span>.
            </p>
          </div>
          <div className="rise rise-2 hairline-t pt-8">
            <Eyebrow>Who sees what</Eyebrow>
            <dl className="mt-4 grid gap-5 text-[14px] sm:grid-cols-2">
              {[
                ["The source chain", "A transfer from the payer to the swap rail."],
                ["The rail", "The payer's address and the Zcash destination. Never a memo."],
                ["Zcash", "One shielded output of unknown amount and origin."],
                ["The embedding site", "Nothing."],
              ].map(([k, v]) => (
                <div key={k}>
                  <dt className="serif text-[20px]">{k}</dt>
                  <dd className="mt-1 text-muted-foreground">{v}</dd>
                </div>
              ))}
            </dl>
          </div>
        </div>
        <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
          <Panel className="flex flex-col gap-4">
            <Eyebrow>Live · mainnet rails</Eyebrow>
            <p className="serif text-[26px] leading-tight">Tip 0.01 ZEC, privately.</p>
            <p className="text-[14px] leading-relaxed text-muted-foreground">Pays this service&apos;s operator address with real funds. Small amounts only.</p>
            <GoDarkDemo amountZats="1000000" label="Tip 0.01 ZEC privately" />
          </Panel>
        </aside>
      </section>
    </div>
  );
}
