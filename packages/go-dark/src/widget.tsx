import { useEffect, useRef, useState } from "react";
import { fetchFreshAddress, fetchQuote, fetchStatus, postEnvelope, submitDeposit } from "./api";
import { connectEthereum, sendErc20ViaRouter, sendEthNative } from "./chains/ethereum";
import { connectSolana, sendSolana } from "./chains/solana";
import type { Asset, Chain, GoDarkOptions, Quote, StatusResponse } from "./types";

const SOLANA_RPC = "https://api.mainnet-beta.solana.com";

type Step = "closed" | "pick" | "quote" | "sending" | "tracking" | "done" | "error";

const ASSETS: Record<Chain, Asset[]> = { solana: ["USDC", "SOL"], ethereum: ["ETH", "USDC"] };

function zec(zats: string | bigint): string {
  const v = BigInt(zats);
  const whole = v / 100_000_000n;
  const frac = (v % 100_000_000n).toString().padStart(8, "0").replace(/0+$/, "");
  return `${whole}${frac ? "." + frac : ""} ZEC`;
}

const css = `
.gd{font:14px/1.5 ui-sans-serif,system-ui,sans-serif;color:#231b14;--gd-accent:#a8542c;--gd-border:rgba(35,27,20,.3);--gd-hair:rgba(35,27,20,.12);--gd-muted:#6e6153;--gd-bg:#faf4ea;--gd-ink:#231b14;--gd-paper:#f3ebdf}
@media(prefers-color-scheme:dark){.gd{color:#f1e7d8;--gd-accent:#d99a5b;--gd-border:rgba(241,231,216,.3);--gd-hair:rgba(241,231,216,.11);--gd-muted:#a69583;--gd-bg:#231b16;--gd-ink:#f1e7d8;--gd-paper:#1b1511}}
.gd *{box-sizing:border-box}
.gd-btn{display:inline-flex;align-items:center;justify-content:center;gap:8px;min-height:44px;padding:0 20px;border-radius:2px;border:1px solid var(--gd-border);background:transparent;color:inherit;font:inherit;font-size:13px;font-weight:500;letter-spacing:.02em;cursor:pointer;transition:opacity .15s ease-out,background-color .15s ease-out}
.gd-btn:hover{background:rgba(127,127,127,.08)}
.gd-btn:focus-visible{outline:2px solid var(--gd-accent);outline-offset:2px}
.gd-btn[disabled]{opacity:.4;cursor:not-allowed}
.gd-btn.primary{background:var(--gd-ink);border-color:var(--gd-ink);color:var(--gd-paper)}
.gd-btn.primary:hover{opacity:.9;background:var(--gd-ink)}
.gd-panel{margin-top:12px;padding:24px;border:1px solid var(--gd-hair);border-radius:6px;background:var(--gd-bg);max-width:440px;display:flex;flex-direction:column;gap:16px}
.gd-row{display:flex;gap:8px;flex-wrap:wrap}
.gd-eyebrow{font-size:11px;letter-spacing:.14em;text-transform:uppercase;color:var(--gd-muted)}
.gd-muted{color:var(--gd-muted);font-size:13px;line-height:1.5}
.gd-mono{font-family:ui-monospace,SFMono-Regular,monospace;font-size:12px;word-break:break-all;font-variant-numeric:tabular-nums}
.gd-seg{display:flex;gap:0;border:1px solid var(--gd-border);border-radius:2px;overflow:hidden}
.gd-seg button{flex:1;min-height:40px;border:0;background:transparent;color:var(--gd-muted);font:inherit;font-size:13px;cursor:pointer;transition:background-color .15s ease-out,color .15s ease-out}
.gd-seg button+button{border-left:1px solid var(--gd-hair)}
.gd-seg button[aria-pressed=true]{background:var(--gd-ink);color:var(--gd-paper)}
.gd-seg button:focus-visible{outline:2px solid var(--gd-accent);outline-offset:-2px}
.gd-table{width:100%;border-collapse:collapse;font-size:13px}
.gd-table td{padding:8px 0;vertical-align:top;border-top:1px solid var(--gd-hair)}
.gd-table td:first-child{color:var(--gd-muted);width:38%;font-size:11px;letter-spacing:.1em;text-transform:uppercase;padding-top:10px}
.gd-err{color:#b4382a}
.gd-title{font-size:18px;line-height:1.25}
`;

export function GoDark(opts: GoDarkOptions) {
  const apiBase = opts.apiBase ?? (typeof window !== "undefined" ? window.location.origin : "");
  const chains = opts.chains ?? ["solana", "ethereum"];
  const [step, setStep] = useState<Step>("closed");
  const [chain, setChain] = useState<Chain>(chains[0]);
  const [asset, setAsset] = useState<Asset>(ASSETS[chains[0]][0]);
  const [account, setAccount] = useState<string | null>(null);
  const [dest, setDest] = useState<{ ua: string; transparent?: string }>({ ua: opts.to ?? "", transparent: opts.toTransparent });
  const [quote, setQuote] = useState<Quote | null>(null);
  const [txHash, setTxHash] = useState<string | null>(null);
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [receipt, setReceipt] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const poll = useRef<number | null>(null);

  const emit = (e: Parameters<NonNullable<GoDarkOptions["onStatus"]>>[0]) => opts.onStatus?.(e);

  useEffect(() => () => { if (poll.current) window.clearInterval(poll.current); }, []);

  function fail(e: unknown) {
    const message = e instanceof Error ? e.message : String(e);
    setError(message);
    setStep("error");
    emit({ type: "error", message });
  }

  async function connectAndQuote() {
    setBusy(true);
    setError(null);
    try {
      const acct = chain === "solana" ? await connectSolana() : await connectEthereum();
      setAccount(acct);
      let d = dest;
      if (opts.toEndpoint) {
        d = await fetchFreshAddress(opts.toEndpoint);
        setDest(d);
      }
      if (!d.ua) throw new Error("no destination: set `to` or `toEndpoint`");
      const q = await fetchQuote(apiBase, {
        chain,
        asset,
        zecOutZats: BigInt(opts.amountZats).toString(),
        recipient: d.ua,
        recipientTransparent: d.transparent,
        refundTo: acct,
      });
      setQuote(q);
      setStep("quote");
      emit({ type: "quoted", quote: q });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  }

  async function pay() {
    if (!quote || !account) return;
    if (quote.expiresAt * 1000 < Date.now() + 15_000) {
      setError("That quote expired. Fetching a fresh one.");
      await connectAndQuote();
      return;
    }
    setBusy(true);
    setStep("sending");
    try {
      let hash: string;
      if (quote.chain === "solana") {
        hash = await sendSolana(SOLANA_RPC, quote.asset as "SOL" | "USDC", quote.depositAddress, BigInt(quote.amountIn));
      } else if (quote.maya?.tokenContract) {
        hash = await sendErc20ViaRouter(
          account,
          quote.maya.router,
          quote.maya.inboundAddress,
          quote.maya.tokenContract,
          BigInt(quote.amountIn),
          quote.maya.memo,
          quote.expiresAt,
        );
      } else {
        hash = await sendEthNative(account, quote.depositAddress, BigInt(quote.amountIn), quote.maya?.memo ?? "");
      }
      setTxHash(hash);
      emit({ type: "sent", chain: quote.chain, txHash: hash, quote });
      const trackingId = quote.rail === "oneclick" ? quote.depositAddress : hash;
      await submitDeposit(apiBase, quote.rail, quote.depositAddress, hash);
      if (opts.envelope) {
        const r = await postEnvelope(opts.envelope.auctionApi, opts.envelope.auctionId, {
          deposit_address: trackingId,
          rail: quote.rail,
          payload: opts.envelope.payload,
        });
        setReceipt(r.receipt);
      }
      setStep("tracking");
      poll.current = window.setInterval(async () => {
        try {
          const s = await fetchStatus(apiBase, quote.rail, trackingId);
          setStatus(s);
          emit({ type: "status", status: s });
          if (s.status === "success" || s.status === "refunded" || s.status === "failed") {
            if (poll.current) window.clearInterval(poll.current);
            if (s.status === "success" && opts.envelope && s.destinationTxids[0]) {
              await postEnvelope(opts.envelope.auctionApi, opts.envelope.auctionId, {
                deposit_address: trackingId,
                rail: quote.rail,
                payload: opts.envelope.payload,
                destination_txid: s.destinationTxids[0],
              }).catch(() => undefined);
            }
            setStep("done");
          }
        } catch {
          /* transient; keep polling */
        }
      }, 10_000);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  }

  function reset() {
    if (poll.current) window.clearInterval(poll.current);
    setQuote(null);
    setTxHash(null);
    setStatus(null);
    setReceipt(null);
    setError(null);
    setStep("pick");
  }

  return (
    <div className="gd">
      <style>{css}</style>
      {step === "closed" ? (
        <button type="button" className="gd-btn primary" onClick={() => setStep("pick")}>
          {opts.label ?? "Pay privately in ZEC"}
        </button>
      ) : (
        <div className="gd-panel" role="dialog" aria-label="Go Dark: pay privately in ZEC">
          <div>
            <div className="gd-eyebrow">Go Dark</div>
            <div className="gd-title">Pay privately.</div>
            <div className="gd-muted">
              Deliver {zec(opts.amountZats)} shielded to{" "}
              {dest.ua ? <span className="gd-mono">{dest.ua.slice(0, 12)}…</span> : "a fresh address created for this payment"}
            </div>
          </div>

          {step === "pick" && (
            <>
              <div className="gd-seg" role="group" aria-label="Source chain">
                {chains.map((c) => (
                  <button
                    key={c}
                    type="button"
                    aria-pressed={chain === c}
                    onClick={() => {
                      setChain(c);
                      setAsset(ASSETS[c][0]);
                    }}
                  >
                    {c === "solana" ? "Solana" : "Ethereum"}
                  </button>
                ))}
              </div>
              <div className="gd-seg" role="group" aria-label="Asset">
                {ASSETS[chain].map((a) => (
                  <button key={a} type="button" aria-pressed={asset === a} onClick={() => setAsset(a)}>
                    {a}
                  </button>
                ))}
              </div>
              <div className="gd-row">
                <button type="button" className="gd-btn primary" disabled={busy} onClick={connectAndQuote}>
                  {busy ? "Getting quote…" : chain === "solana" ? "Connect Phantom" : "Connect MetaMask"}
                </button>
                <button type="button" className="gd-btn" onClick={() => setStep("closed")}>
                  Cancel
                </button>
              </div>
            </>
          )}

          {step === "quote" && quote && (
            <>
              <table className="gd-table">
                <tbody>
                  <tr>
                    <td>You send</td>
                    <td className="gd-mono">{quote.amountInFormatted}</td>
                  </tr>
                  <tr>
                    <td>Recipient gets</td>
                    <td className="gd-mono">{zec(quote.amountOutZats)} shielded</td>
                  </tr>
                  <tr>
                    <td>Quote valid</td>
                    <td>{Math.max(0, Math.round((quote.expiresAt * 1000 - Date.now()) / 60000))} min</td>
                  </tr>
                  <tr>
                    <td>Rail</td>
                    <td>{quote.rail === "maya" ? "Maya Protocol" : "NEAR Intents"} · about {Math.ceil(quote.etaSeconds / 60)} min</td>
                  </tr>
                </tbody>
              </table>
              <table className="gd-table" aria-label="Who sees what">
                <tbody>
                  <tr>
                    <td>{chain === "solana" ? "Solana" : "Ethereum"} sees</td>
                    <td>A transfer from your wallet to the swap rail.</td>
                  </tr>
                  <tr>
                    <td>The rail sees</td>
                    <td>Your source address and the Zcash destination address. Not the memo.</td>
                  </tr>
                  <tr>
                    <td>Zcash sees</td>
                    <td>
                      {quote.delivery === "shielded"
                        ? "One shielded output. No amount, no sender."
                        : "A transparent payment to a one-time address, shielded by the recipient on arrival. The amount is visible for that one hop."}
                    </td>
                  </tr>
                  <tr>
                    <td>Still visible</td>
                    <td>Your {chain === "solana" ? "Solana" : "Ethereum"} wallet sent funds to a swap rail at this time. Use a fresh source wallet if that link matters to you.</td>
                  </tr>
                  <tr>
                    <td>This site sees</td>
                    <td>Nothing. Quotes are proxied without logging addresses.</td>
                  </tr>
                </tbody>
              </table>
              <div className="gd-row">
                <button type="button" className="gd-btn primary" disabled={busy} onClick={pay}>
                  Confirm in wallet
                </button>
                <button type="button" className="gd-btn" onClick={reset}>
                  Back
                </button>
              </div>
            </>
          )}

          {step === "sending" && <div>Waiting for your wallet…</div>}

          {(step === "tracking" || step === "done") && (
            <>
              <div>
                {status?.status === "success"
                  ? "Delivered. The ZEC landed shielded."
                  : status?.status === "refunded"
                    ? "The rail refunded your deposit."
                    : status?.status === "failed"
                      ? "The swap failed. Contact the rail with your tx hash."
                      : "Swap in progress. This usually takes a few minutes."}
              </div>
              {txHash && (
                <div className="gd-muted">
                  Source tx <span className="gd-mono">{txHash}</span>
                </div>
              )}
              {status?.destinationTxids[0] && (
                <div className="gd-muted">
                  Zcash tx <span className="gd-mono">{status.destinationTxids[0]}</span>
                </div>
              )}
              {receipt && (
                <div className="gd-muted">
                  Bid receipt <span className="gd-mono">{receipt}</span>
                </div>
              )}
              <div className="gd-row">
                <button type="button" className="gd-btn" onClick={() => setStep("closed")}>
                  Close
                </button>
              </div>
            </>
          )}

          {step === "error" && (
            <>
              <div className="gd-err" role="alert">
                {error}
              </div>
              <div className="gd-row">
                <button type="button" className="gd-btn primary" onClick={reset}>
                  Try again
                </button>
                <button type="button" className="gd-btn" onClick={() => setStep("closed")}>
                  Close
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
