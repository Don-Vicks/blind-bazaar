"use client";

import { useState } from "react";
import { Button, Eyebrow, Mono } from "@/components/ui";
import { generateReceiverWallet, type ReceiverWallet } from "@/lib/receiver-wallet";

/** Make a Zcash wallet in this browser for someone who wants to receive shielded ZEC and has none. The phrase
 *  is shown once, never stored, never sent; the address goes into the caller's form. */
export function ReceiverWalletMaker({
  network,
  birthday,
  onAddress,
}: {
  network: "main" | "test";
  /** Current chain height: the wallet's birthday, so an import does not rescan the whole chain. */
  birthday: number | null;
  onAddress: (address: string) => void;
}) {
  const [wallet, setWallet] = useState<ReceiverWallet | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [copied, setCopied] = useState<"phrase" | "address" | null>(null);

  async function make() {
    setBusy(true);
    setError(null);
    try {
      const w = await generateReceiverWallet(network);
      setWallet(w);
      setSaved(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copy(what: "phrase" | "address", text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(what);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      /* clipboard unavailable */
    }
  }

  if (!wallet) {
    return (
      <div className="rounded-md border border-hairline bg-surface p-4 text-[13px] text-muted-foreground">
        <p>
          No Zcash wallet yet? This page can make one. Twenty-four words are generated here in your browser, with the same key
          derivation the mobile wallets use, and never leave the page. You get an address to receive at now and import the words
          into Zodl, Zingo or YWallet whenever you want to spend.
        </p>
        <div className="mt-3 flex items-center gap-3">
          <Button variant="secondary" onClick={make} disabled={busy}>
            {busy ? "Making…" : "Make a wallet in this browser"}
          </Button>
          {error && <span className="text-danger">{error}</span>}
        </div>
      </div>
    );
  }

  const words = wallet.phrase.split(" ");
  return (
    <div className="rounded-md border border-accent bg-surface p-4 text-[13px]">
      <Eyebrow>Your new wallet. Write these words down now.</Eyebrow>
      <p className="mt-2 text-muted-foreground">
        They are the only way to spend what this address receives. This page does not keep them; close it and they are gone.
      </p>
      <ol className="num mt-3 grid grid-cols-2 gap-x-6 gap-y-1 text-[13px] sm:grid-cols-3">
        {words.map((w, i) => (
          <li key={i} className="flex gap-2">
            <span className="w-6 text-right text-muted-foreground">{i + 1}.</span>
            <span>{w}</span>
          </li>
        ))}
      </ol>
      <div className="mt-3 flex flex-wrap items-center gap-3">
        <Button variant="secondary" onClick={() => copy("phrase", wallet.phrase)}>
          {copied === "phrase" ? "Copied" : "Copy the words"}
        </Button>
        <label className="flex cursor-pointer items-center gap-2 text-muted-foreground">
          <input type="checkbox" checked={saved} onChange={(e) => setSaved(e.target.checked)} className="accent-[var(--accent)]" />I have written them down
        </label>
      </div>
      <div className={`mt-4 ${saved ? "" : "pointer-events-none opacity-40"}`}>
        <p className="text-foreground">Your receiving address</p>
        <Mono className="mt-1 block text-muted-foreground">{wallet.address}</Mono>
        <p className="mt-2 text-muted-foreground">
          A unified address with a shielded receiver and a transparent receiver, so every rail can pay it; shielded rails use the
          shielded one. Wallets show it slightly differently after import, which is normal: it is the same key.
        </p>
        <details className="mt-2 text-muted-foreground">
          <summary className="cursor-pointer hover:text-foreground">The two receivers on their own</summary>
          <p className="mt-1">Shielded only (what a lot address looks like):</p>
          <Mono className="block">{wallet.orchardOnly}</Mono>
          <p className="mt-1">Transparent only (for a rail that cannot pay a unified address; the amount is public until you shield it):</p>
          <Mono className="block" data-testid="receiver-transparent">{wallet.transparent}</Mono>
        </details>
        <div className="mt-3 flex flex-wrap items-center gap-3">
          <Button onClick={() => onAddress(wallet.address)}>Use this address</Button>
          <Button variant="secondary" onClick={() => copy("address", wallet.address)}>
            {copied === "address" ? "Copied" : "Copy address"}
          </Button>
        </div>
        <details className="mt-4 text-muted-foreground">
          <summary className="cursor-pointer hover:text-foreground">To spend: import into a mobile wallet</summary>
          <ol className="mt-2 list-decimal space-y-1 pl-5">
            <li>Install Zodl (formerly Zashi), Zingo or YWallet{network === "test" ? " (a testnet build: Zodl has none, Zingo and YWallet do)" : ""}.</li>
            <li>Choose restore or import and enter the twenty-four words in order.</li>
            <li>
              When asked for a birthday or start height, enter <span className="num text-foreground">{birthday ? birthday.toLocaleString() : "the current block height"}</span>. The wallet then scans only from today.
            </li>
            <li>Funds sent to the address above appear once the wallet has scanned past the block they were mined in.</li>
          </ol>
        </details>
      </div>
      <p className="mt-4 text-[12px] text-muted-foreground">
        Network: {network === "main" ? "Zcash mainnet" : "Zcash testnet"}. Made with the same derivation the auction service uses (checked by test), transparent receiver by BIP 44, no network call, nothing stored.
      </p>
    </div>
  );
}
