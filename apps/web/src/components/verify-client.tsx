"use client";

import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, explorerTxUrl } from "@/lib/api";
import { zec } from "@/lib/format";
import { loadVerifier } from "@/lib/verify-wasm";
import { Eyebrow, LinkButton, Mono, Panel, Skeleton } from "@/components/ui";

interface Disclosure {
  txid: string;
  action_index: number;
  pool: string;
  recipient_address: string;
  orchard_receiver: string;
  value_zats: number;
  rho: string;
  rseed: string;
  note_version: number;
  memo: string | null;
  cmx: string;
  verified_against_chain: boolean;
  enc_key?: string;
  enc_ciphertext?: string;
  reserve_zats?: number;
  reserve_rule?: "at_least" | "at_most";
  bid_zats?: number;
}

interface Check {
  d: Disclosure;
  computedCmx?: string;
  cmxMatches?: boolean;
  memo?: string | null;
  bidZats?: number;
  reserveMet?: boolean;
  error?: string;
}

export function VerifyClient({ id }: { id: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const { data, isPending, isError } = useQuery({ queryKey: ["result", id], queryFn: () => api.result(id), retry: false });
  const [checks, setChecks] = useState<Check[] | null>(null);
  const [wasmError, setWasmError] = useState<string | null>(null);
  const network = health.data?.network ?? "test";

  useEffect(() => {
    if (!data?.disclosure) return;
    const list = (Array.isArray(data.disclosure) ? data.disclosure : [data.disclosure]) as Disclosure[];
    let cancelled = false;
    (async () => {
      try {
        const v = await loadVerifier();
        const out: Check[] = list.map((d) => {
          const c: Check = { d };
          try {
            c.computedCmx = v.compute_cmx(d.orchard_receiver, BigInt(d.value_zats), d.rho, d.rseed, d.note_version);
            c.cmxMatches = c.computedCmx === d.cmx;
            if (d.enc_key && d.enc_ciphertext) {
              const note = JSON.parse(v.decrypt_note(d.enc_key, d.enc_ciphertext)) as { value: number; rseed: string; memo: string | null };
              if (note.value !== d.value_zats || note.rseed !== d.rseed) throw new Error("decrypted note does not match the opening");
              c.memo = note.memo;
              const m = /^BB1\|[0-9a-z]{8}\|(\d+)\|/.exec(note.memo ?? "");
              if (m) {
                c.bidZats = Number(m[1]);
                if (d.reserve_zats !== undefined) c.reserveMet = d.reserve_rule === "at_most" ? c.bidZats <= d.reserve_zats : c.bidZats >= d.reserve_zats;
              }
            }
          } catch (e) {
            c.error = (e as Error).message;
          }
          return c;
        });
        if (!cancelled) setChecks(out);
      } catch (e) {
        if (!cancelled) setWasmError((e as Error).message);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [data]);

  if (isPending) return <Skeleton className="h-64" />;
  if (isError || !data) {
    return (
      <div className="flex flex-col items-start gap-4">
        <p className="serif text-[28px]">No result to verify yet.</p>
        <LinkButton href={`/a/${id}`} variant="secondary">
          Back to the lot
        </LinkButton>
      </div>
    );
  }

  const all = checks && checks.length > 0 && checks.every((c) => c.cmxMatches && !c.error);

  return (
    <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
      <div className="flex flex-col gap-10 sm:col-span-7">
        <div className="rise">
          <Eyebrow>Lot {id} · verify in your browser</Eyebrow>
          <h1 className="serif mt-4 text-[44px] leading-[1.02] sm:text-[64px]">
            {checks === null && !wasmError ? "Checking…" : all ? "The opening matches what the service reports." : wasmError ? "Verifier unavailable." : "Something does not match."}
          </h1>
          <p className="dropcap mt-8 max-w-[52ch] text-[17px] leading-[1.6] text-muted-foreground">
            Your browser rebuilds each winning note from the published opening and recomputes its commitment with the
            same Orchard code the chain uses, compiled to WebAssembly. It then compares that commitment with the one
            the service reports for the transaction. To be certain the service did not lie about the chain, open the
            transaction on an explorer and compare the commitment yourself; the link is below.
          </p>
        </div>
        {wasmError && <p className="text-[14px] text-danger">{wasmError}</p>}
        <ol className="flex flex-col gap-6">
          {(checks ?? []).map((c, i) => (
            <li key={`${c.d.txid}-${c.d.action_index}`} className="hairline-t pt-6">
              <div className="flex items-baseline justify-between gap-4">
                <p className="serif text-[22px]">
                  Output {i + 1} · action {c.d.action_index} · {zec(c.d.value_zats)}
                </p>
                <span className={`text-[13px] ${c.error ? "text-danger" : c.cmxMatches ? "text-success" : "text-danger"}`}>
                  {c.error ? "error" : c.cmxMatches ? "commitment matches" : "mismatch"}
                </span>
              </div>
              <dl className="mt-3 grid gap-2 text-[13px]">
                <div>
                  <dt className="eyebrow">Recomputed in your browser</dt>
                  <dd><Mono>{c.computedCmx ?? "—"}</Mono></dd>
                </div>
                <div>
                  <dt className="eyebrow">Reported from the transaction</dt>
                  <dd><Mono className="text-muted-foreground">{c.d.cmx}</Mono></dd>
                </div>
                <div>
                  <dt className="eyebrow">Transaction</dt>
                  <dd>
                    <a href={explorerTxUrl(network, c.d.txid)} target="_blank" rel="noreferrer" className="num break-all underline decoration-hairline-strong underline-offset-4 hover:decoration-accent">
                      {c.d.txid}
                    </a>
                  </dd>
                </div>
                {c.memo !== undefined && (
                  <div>
                    <dt className="eyebrow">Memo, decrypted here with the disclosed key</dt>
                    <dd><Mono>{c.memo ?? "(empty)"}</Mono></dd>
                    {c.bidZats !== undefined && c.d.reserve_zats !== undefined && (
                      <p className={`mt-1 ${c.reserveMet ? "text-success" : "text-danger"}`}>
                        {c.d.reserve_rule === "at_most"
                          ? `Quote ${zec(c.bidZats)} against budget ${zec(c.d.reserve_zats)}: ${c.reserveMet ? "within budget" : "over budget"}`
                          : `Bid ${zec(c.bidZats)} against reserve ${zec(c.d.reserve_zats)}: ${c.reserveMet ? "reserve met" : "below reserve"}`}
                      </p>
                    )}
                  </div>
                )}
                {c.error && <p className="text-danger">{c.error}</p>}
              </dl>
            </li>
          ))}
        </ol>
      </div>
      <aside className="rise rise-2 sm:col-span-5 sm:sticky sm:top-8 sm:self-start">
        <Panel className="flex flex-col gap-5">
          <div>
            <Eyebrow>What this proves</Eyebrow>
            <p className="serif mt-2 text-[26px] leading-tight">One output, nothing else. For a chain check, run the command.</p>
          </div>
          <p className="text-[14px] leading-relaxed text-muted-foreground">
            A note opening lets anyone confirm the recipient and value of exactly one shielded output. The viewing key
            stays private, and no losing bid can be opened from it. If a disclosed memo key is present, that single
            note&apos;s memo is readable too, which is how the reserve is proven.
          </p>
          <div className="hairline-t pt-4 text-[13px] text-muted-foreground">
            <p>Command-line equivalent:</p>
            <Mono className="mt-2 block">bb-auction --verify-disclosure disclosure.json</Mono>
          </div>
          <LinkButton href={`/a/${id}/result`} variant="secondary">
            Back to the result
          </LinkButton>
        </Panel>
      </aside>
    </div>
  );
}
