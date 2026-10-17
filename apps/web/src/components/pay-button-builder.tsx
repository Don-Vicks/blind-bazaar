"use client";

import { GoDark } from "@blind-bazaar/go-dark";
import { looksLikeShieldedAddress, parseZec } from "@blind-bazaar/sdk";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api";
import { Button, Eyebrow, Field, Mono, Panel, inputClass } from "@/components/ui";

/** Configure a pay button, see it live, copy the snippet. */
export function PayButtonBuilder({ site }: { site: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const [label, setLabel] = useState("Pay 0.01 ZEC privately");
  const [amount, setAmount] = useState("0.01");
  const [mode, setMode] = useState<"fresh" | "own">("fresh");
  const [own, setOwn] = useState("");
  const [copied, setCopied] = useState(false);
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);

  const zats = useMemo(() => {
    try {
      return parseZec(amount);
    } catch {
      return 0n;
    }
  }, [amount]);
  const amountOk = zats >= 10_000n;
  const ownOk = mode === "own" ? looksLikeShieldedAddress(own.trim()) : true;

  const snippet = `<script src="${site}/go-dark.iife.js"></script>
<div id="pay"></div>
<script>
  GoDark.mount(document.getElementById("pay"), {
${mode === "fresh" ? `    toEndpoint: "${site}/api/godark/address", // a fresh address per payment` : `    to: "${own.trim() || "u1…"}", // your unified address`}
    amountZats: "${zats.toString()}", // ${amount || "0"} ZEC, delivered shielded
    apiBase: "${site}",
    label: ${JSON.stringify(label)},
  });
</script>`;

  async function copy() {
    try {
      await navigator.clipboard.writeText(snippet);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable */
    }
  }

  const previewTo = mode === "own" ? own.trim() : health.data?.operator_address;

  return (
    <div className="grid gap-8 sm:grid-cols-12 sm:gap-10">
      <Panel className="flex flex-col gap-5 sm:col-span-5">
        <div>
          <Eyebrow>Build your button</Eyebrow>
          <p className="serif mt-2 text-[26px] leading-tight">Three choices.</p>
        </div>
        <Field label="Button text" htmlFor="pb-label">
          <input id="pb-label" className={inputClass} value={label} onChange={(e) => setLabel(e.target.value)} />
        </Field>
        <Field label="Amount (ZEC)" htmlFor="pb-amount" error={amountOk ? undefined : "At least 0.0001 ZEC."} hint="What the recipient receives, shielded. The payer's side is quoted live.">
          <input id="pb-amount" inputMode="decimal" className={`${inputClass} num`} value={amount} onChange={(e) => setAmount(e.target.value)} />
        </Field>
        <fieldset className="flex flex-col gap-2">
          <legend className="eyebrow mb-2 text-foreground">Where the ZEC goes</legend>
          <label className={`flex cursor-pointer items-start gap-3 rounded-[2px] border px-3 py-3 text-[14px] ${mode === "fresh" ? "border-accent" : "border-hairline-strong"}`}>
            <input type="radio" name="pb-mode" checked={mode === "fresh"} onChange={() => setMode("fresh")} className="mt-1 accent-[var(--accent)]" />
            <span>
              A fresh address per payment
              <span className="block text-[13px] text-muted-foreground">This service creates a new address for every payer, so two payments to you cannot be linked. Funds land in the service wallet it manages for you.</span>
            </span>
          </label>
          <label className={`flex cursor-pointer items-start gap-3 rounded-[2px] border px-3 py-3 text-[14px] ${mode === "own" ? "border-accent" : "border-hairline-strong"}`}>
            <input type="radio" name="pb-mode" checked={mode === "own"} onChange={() => setMode("own")} className="mt-1 accent-[var(--accent)]" />
            <span>
              My own Zcash address
              <span className="block text-[13px] text-muted-foreground">Paste a unified address from your wallet. Every payment lands there directly. No account with us.</span>
            </span>
          </label>
        </fieldset>
        {mode === "own" && (
          <Field label="Your unified address" htmlFor="pb-own" error={own && !ownOk ? "A unified address, starting with u1 or utest1." : undefined}>
            <input id="pb-own" spellCheck={false} placeholder="u1…" className={`${inputClass} num text-[13px]`} value={own} onChange={(e) => setOwn(e.target.value)} />
          </Field>
        )}
      </Panel>

      <div className="flex flex-col gap-8 sm:col-span-7">
        <div className="rounded-md border border-hairline p-6">
          <Eyebrow>Live preview</Eyebrow>
          <p className="mt-1 text-[13px] text-muted-foreground">This is the real button, on {health.data?.network === "main" ? "mainnet" : "mainnet rails with a testnet recipient; do not send"}. Click it to see what a payer sees.</p>
          <div className="mt-4">
            {mounted && previewTo && amountOk && ownOk ? (
              <GoDark key={`${mode}-${previewTo}-${zats}-${label}`} to={previewTo} toEndpoint={mode === "fresh" ? "/api/godark/address" : undefined} amountZats={zats.toString()} label={label} />
            ) : (
              <p className="text-[13px] text-muted-foreground">{!amountOk ? "Enter an amount to preview." : !ownOk ? "Enter a valid address to preview." : "Waiting for the service…"}</p>
            )}
          </div>
        </div>
        <div>
          <div className="flex items-baseline justify-between">
            <Eyebrow>Paste this into your page</Eyebrow>
            <Button variant="secondary" onClick={copy} className="min-h-9 px-3 text-[12px]">
              {copied ? "Copied" : "Copy snippet"}
            </Button>
          </div>
          <pre className="num mt-3 overflow-x-auto rounded-md border border-hairline bg-surface p-4 text-[12px] leading-relaxed text-muted-foreground">{snippet}</pre>
          <p className="mt-3 text-[13px] text-muted-foreground">
            React instead: <Mono className="text-foreground">import {"{ GoDark }"} from &quot;@blind-bazaar/go-dark&quot;</Mono> with the same options.
          </p>
        </div>
      </div>
    </div>
  );
}
