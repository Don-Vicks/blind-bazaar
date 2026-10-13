import { useEffect, useRef, useState } from "react";
import { fetchQuote, fetchStatus, postEnvelope, submitDeposit } from "./api";
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
.gd{font:14px/1.45 system-ui,sans-serif;color:#1a1714;--gd-accent:#b4551a;--gd-border:#e2dcd0;--gd-muted:#6b645a;--gd-bg:#fff}
@media(prefers-color-scheme:dark){.gd{color:#ece7dd;--gd-accent:#e8824a;--gd-border:#2a2722;--gd-muted:#a39b8e;--gd-bg:#15130f}}
.gd *{box-sizing:border-box}
.gd-btn{display:inline-flex;align-items:center;gap:8px;min-height:40px;padding:0 16px;border-radius:8px;border:1px solid var(--gd-border);background:var(--gd-bg);color:inherit;font:inherit;font-weight:500;cursor:pointer}
.gd-btn:focus-visible{outline:2px solid var(--gd-accent);outline-offset:2px}
.gd-btn[disabled]{opacity:.5;cursor:not-allowed}
.gd-btn.primary{background:var(--gd-accent);border-color:var(--gd-accent);color:#fff}
.gd-panel{margin-top:12px;padding:16px;border:1px solid var(--gd-border);border-radius:12px;background:var(--gd-bg);max-width:420px;display:flex;flex-direction:column;gap:12px}
.gd-row{display:flex;gap:8px;flex-wrap:wrap}
.gd-muted{color:var(--gd-muted);font-size:12px}
.gd-mono{font-family:ui-monospace,SFMono-Regular,monospace;font-size:12px;word-break:break-all}
.gd-seg{display:flex;gap:4px;padding:4px;border:1px solid var(--gd-border);border-radius:8px}
.gd-seg button{flex:1;min-height:36px;border:0;border-radius:6px;background:transparent;color:inherit;font:inherit;cursor:pointer}
.gd-seg button[aria-pressed=true]{background:var(--gd-accent);color:#fff}
.gd-seg button:focus-visible{outline:2px solid var(--gd-accent)}
.gd-table{width:100%;border-collapse:collapse;font-size:12px}
.gd-table td{padding:4px 0;vertical-align:top;border-top:1px solid var(--gd-border)}
.gd-table td:first-child{color:var(--gd-muted);width:38%}
.gd-err{color:#b42318}
`;

export function GoDark(opts: GoDarkOptions) {
  const apiBase = opts.apiBase ?? (typeof window !== "undefined" ? window.location.origin : "");
  const chains = opts.chains ?? ["solana", "ethereum"];
  const [step, setStep] = useState<Step>("closed");
  const [chain, setChain] = useState<Chain>(chains[0]);
  const [asset, setAsset] = useState<Asset>(ASSETS[chains[0]][0]);
  const [account, setAccount] = useState<string | null>(null);
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
      const q = await fetchQuote(apiBase, {
        chain,
        asset,
        zecOutZats: BigInt(opts.amountZats).toString(),
        recipient: opts.to,
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
            <strong>Pay privately</strong>
            <div className="gd-muted">
              Deliver {zec(opts.amountZats)} shielded to <span className="gd-mono">{opts.to.slice(0, 12)}…</span>
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
                    <td>One shielded output. No amount, no sender.</td>
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
