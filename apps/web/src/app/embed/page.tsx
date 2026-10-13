import { Card } from "@/components/ui";

export default function EmbedPage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Go Dark</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          A button any site can embed to receive shielded ZEC from a Solana or Ethereum wallet. Coming in this build.
        </p>
      </div>
      <Card>
        <pre className="overflow-x-auto font-mono text-xs">{`<script src="https://cdn.example/go-dark.iife.js"></script>
<div id="pay"></div>
<script>GoDark.mount(document.getElementById("pay"), { to: "u1…", amount: { zec: 0.01 } })</script>`}</pre>
      </Card>
    </div>
  );
}
