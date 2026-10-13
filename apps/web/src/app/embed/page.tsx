import { Card } from "@/components/ui";
import { GoDarkDemo } from "@/components/go-dark-demo";

export default function EmbedPage() {
  const snippet = `<script src="${process.env.NEXT_PUBLIC_SITE_URL ?? "https://blindbazaar.example"}/go-dark.iife.js"></script>
<div id="pay"></div>
<script>
  GoDark.mount(document.getElementById("pay"), {
    to: "u1…",            // your unified address
    amountZats: "1000000", // 0.01 ZEC, delivered shielded
    apiBase: "${process.env.NEXT_PUBLIC_SITE_URL ?? "https://blindbazaar.example"}",
  });
</script>`;
  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Go Dark</h1>
        <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
          One button any site can embed. A visitor pays from Phantom or MetaMask and the recipient gets shielded ZEC
          at a unified address. Ethereum routes through Maya Protocol, Solana through NEAR Intents. No account, no
          Zcash wallet needed to pay.
        </p>
      </div>
      <Card className="flex flex-col gap-3">
        <h2 className="text-sm font-medium">Live demo (mainnet rails, pays this service&apos;s operator address)</h2>
        <GoDarkDemo amountZats="1000000" label="Tip 0.01 ZEC privately" />
      </Card>
      <Card className="flex flex-col gap-3">
        <h2 className="text-sm font-medium">Embed snippet</h2>
        <pre className="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">{snippet}</pre>
        <p className="text-xs text-muted-foreground">
          React users: <span className="font-mono">import {"{ GoDark }"} from &quot;@blind-bazaar/go-dark&quot;</span>. The
          host page only proxies quotes; it never sees the memo and logs no addresses.
        </p>
      </Card>
      <Card className="flex flex-col gap-2 text-sm">
        <h2 className="font-medium">Who sees what</h2>
        <ul className="list-disc space-y-1 pl-5 text-muted-foreground">
          <li>The source chain sees a transfer from the payer to the swap rail.</li>
          <li>The rail sees the payer&apos;s source address and the Zcash destination. Never a memo.</li>
          <li>Zcash sees one shielded output of unknown amount and origin.</li>
          <li>The embedding site sees nothing.</li>
        </ul>
      </Card>
    </div>
  );
}
