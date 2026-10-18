"use client";

import { looksLikeShieldedAddress, parseZec } from "@blind-bazaar/sdk";
import { useMutation } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";
import { FORMATS, type Format } from "@/lib/formats";
import { Button, Eyebrow, Field, Panel, inputClass } from "@/components/ui";

const TOKEN_KEY = "bb.adminToken";
const SELLER_KEY = "bb.sellerKey";

export default function SellPage() {
  const router = useRouter();
  const [token, setToken] = useState("");
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [reserve, setReserve] = useState("");
  const [deposit, setDeposit] = useState("");
  const [seller, setSeller] = useState("");
  const [hours, setHours] = useState("24");
  const [imageUrl, setImageUrl] = useState("");
  const [units, setUnits] = useState("1");
  const [sealed, setSealed] = useState(true);
  const [revealHours, setRevealHours] = useState("2");
  const [format, setFormat] = useState<Format>("first_price");
  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    try {
      setToken(sessionStorage.getItem(SELLER_KEY) ?? sessionStorage.getItem(TOKEN_KEY) ?? "");
    } catch {
      /* no storage */
    }
  }, []);

  const create = useMutation({
    mutationFn: (body: Record<string, unknown>) => api.createAuction(token, body),
    onSuccess: (a) => router.push(token.startsWith("bbs_") ? `/dashboard/${a.id}` : `/a/${a.id}`),
  });

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const next: Record<string, string> = {};
    let reserveZats = 0n;
    let depositZats = 0n;
    if (!token.trim()) next.token = "Enter your seller key.";
    if (!title.trim()) next.title = "Give the lot a title.";
    try {
      reserveZats = parseZec(reserve);
      if (reserveZats === 0n) next.reserve = "Reserve must be above zero.";
    } catch {
      next.reserve = "Enter an amount in ZEC.";
    }
    try {
      depositZats = parseZec(deposit);
      if (depositZats === 0n) next.deposit = "Deposit must be above zero.";
      if (reserveZats && depositZats > reserveZats) next.deposit = "Deposit should not exceed the reserve.";
    } catch {
      next.deposit = "Enter an amount in ZEC.";
    }
    if (!looksLikeShieldedAddress(seller.trim())) next.seller = "Enter a unified address (u1… or utest1…).";
    const h = Number(hours);
    if (!(h > 0 && h <= 24 * 14)) next.hours = "Between 0.1 and 336 hours.";
    if (imageUrl.trim() && !/^https:\/\/\S+$/.test(imageUrl.trim())) next.image = "An https image URL.";
    const u = format === "uniform" ? Number(units) : 1;
    if (!(Number.isInteger(u) && u >= 1 && u <= 10000)) next.units = "A whole number from 1 to 10000.";
    if (format === "uniform" && u < 2) next.units = "An edition needs at least 2 units.";
    setErrors(next);
    if (Object.keys(next).length) return;
    try {
      sessionStorage.setItem(token.trim().startsWith("bbs_") ? SELLER_KEY : TOKEN_KEY, token.trim());
    } catch {
      /* no storage */
    }
    create.mutate({
      title: title.trim(),
      description: description.trim(),
      reserve_zats: Number(reserveZats),
      deposit_zats: Number(depositZats),
      seller_address: seller.trim(),
      duration_secs: Math.round(h * 3600),
      image_url: imageUrl.trim() || undefined,
      units: format === "uniform" ? Number(units) : 1,
      format,
      reveal_secs: sealed && format !== "rfq" ? Math.round(Number(revealHours) * 3600) : 0,
    });
  }

  const serverError =
    create.error instanceof ApiError
      ? create.error.status === 401
        ? "That key was rejected."
        : create.error.message
      : create.error
        ? "Could not reach the auction service."
        : null;

  return (
    <div className="grid gap-12 sm:grid-cols-12 sm:gap-10">
      <div className="rise flex flex-col gap-8 sm:col-span-5">
        <div>
          <Eyebrow>Sell</Eyebrow>
          <h1 className="serif mt-4 text-[44px] leading-[1.02] sm:text-[56px]">List a lot.</h1>
          <p className="mt-6 max-w-[40ch] text-[16px] leading-relaxed text-muted-foreground">
            The service derives a fresh shielded address for the auction. Bidders pay the deposit to it with their
            sealed bid in the memo. You learn the winning amount and nothing else.
          </p>
        </div>
        <dl className="hairline-t grid gap-4 pt-6 text-[14px]">
          <div><dt className="text-foreground">Sealed first price</dt><dd className="text-muted-foreground">Highest bid wins and pays its bid. Reserve published, deposit returned to losers.</dd></div>
          <div><dt className="text-foreground">Vickrey</dt><dd className="text-muted-foreground">Highest bid wins but pays the second bid. Bidders bid their true value; the winning bid is never revealed, the price is proven from the ledger.</dd></div>
          <div><dt className="text-foreground">Edition</dt><dd className="text-muted-foreground">Identical units. The top N win and everyone pays the same, the lowest winning bid.</dd></div>
          <div><dt className="text-foreground">Request for quotes</dt><dd className="text-muted-foreground">You are buying. Set a budget; suppliers post a bond and seal a quote. Lowest quote wins, you pay it, the winner gets quote plus bond.</dd></div>
        </dl>
      </div>
      <div className="rise rise-2 sm:col-span-7">
      <Panel>
        <form onSubmit={onSubmit} noValidate className="flex flex-col gap-5">
          <Field label="Title" htmlFor="title" error={errors.title}>
            <input id="title" name="title" type="text" autoComplete="off" className={inputClass} value={title} onChange={(e) => setTitle(e.target.value)} aria-invalid={!!errors.title} />
          </Field>
          <Field label="Description" htmlFor="description" hint="Optional. What is being sold and how it is delivered.">
            <textarea id="description" name="description" rows={3} className={`${inputClass} min-h-24 py-2`} value={description} onChange={(e) => setDescription(e.target.value)} />
          </Field>
          <fieldset className="flex flex-col gap-2">
            <legend className="eyebrow mb-2">Format</legend>
            <div className="grid gap-2 sm:grid-cols-2">
              {(Object.keys(FORMATS) as Format[]).map((f) => (
                <label key={f} className={`flex min-h-11 cursor-pointer items-center gap-3 rounded-[2px] border px-3 text-[14px] ${format === f ? "border-accent" : "border-hairline-strong"}`}>
                  <input type="radio" name="format" value={f} checked={format === f} onChange={() => setFormat(f)} className="accent-[var(--accent)]" />
                  {FORMATS[f].name}
                </label>
              ))}
            </div>
            <p className="text-[13px] text-muted-foreground">{FORMATS[format].rule}</p>
          </fieldset>
          {format !== "rfq" && (
            <fieldset className="flex flex-col gap-2">
              <legend className="eyebrow mb-2">Who can read bids before close</legend>
              <label className={`flex cursor-pointer items-start gap-3 rounded-[2px] border px-3 py-3 text-[14px] ${sealed ? "border-accent" : "border-hairline-strong"}`}>
                <input type="radio" name="sealed" checked={sealed} onChange={() => setSealed(true)} className="mt-1 accent-[var(--accent)]" />
                <span>
                  Nobody, not even the auction (sealed-reveal)
                  <span className="block text-[13px] text-muted-foreground">Bids carry a commitment to the amount. After close, bidders reveal within a window; anyone who does not is refunded in full and cannot win.</span>
                </span>
              </label>
              <label className={`flex cursor-pointer items-start gap-3 rounded-[2px] border px-3 py-3 text-[14px] ${!sealed ? "border-accent" : "border-hairline-strong"}`}>
                <input type="radio" name="sealed" checked={!sealed} onChange={() => setSealed(false)} className="mt-1 accent-[var(--accent)]" />
                <span>
                  The auction's viewing key (classic)
                  <span className="block text-[13px] text-muted-foreground">Bids carry the amount; the result is final the moment the lot closes, with no reveal step for bidders.</span>
                </span>
              </label>
              {sealed && (
                <Field label="Reveal window (hours)" htmlFor="reveal-hours" hint="How long bidders have to reveal after close. Bids not revealed in time are refunded.">
                  <input id="reveal-hours" type="number" min="0.1" max="168" step="0.1" className={`${inputClass} num`} value={revealHours} onChange={(e) => setRevealHours(e.target.value)} />
                </Field>
              )}
            </fieldset>
          )}
          <div className="grid gap-5 sm:grid-cols-2">
            <Field label={`${FORMATS[format].floorLabel} (ZEC)`} htmlFor="reserve" error={errors.reserve} hint={format === "rfq" ? "The most you will pay." : "Lowest bid you will accept."}>
              <input id="reserve" name="reserve" type="text" inputMode="decimal" autoComplete="off" placeholder="0.5" className={`${inputClass} num`} value={reserve} onChange={(e) => setReserve(e.target.value)} aria-invalid={!!errors.reserve} />
            </Field>
            <Field label={`${format === "rfq" ? "Bond" : "Deposit"} (ZEC)`} htmlFor="deposit" error={errors.deposit} hint={format === "rfq" ? "Every quoter posts this to quote. Returned with the payout, or when they lose." : "Every bidder pays this to bid. Losers get it back."}>
              <input id="deposit" name="deposit" type="text" inputMode="decimal" autoComplete="off" placeholder="0.05" className={`${inputClass} num`} value={deposit} onChange={(e) => setDeposit(e.target.value)} aria-invalid={!!errors.deposit} />
            </Field>
          </div>
          <Field label={format === "rfq" ? "Your address" : "Payout address"} htmlFor="seller" error={errors.seller} hint={format === "rfq" ? "Your unified address. Any overpayment on settlement returns here shielded." : "Your unified address. The winning amount is sent here shielded."}>
            <input id="seller" name="seller" type="text" autoComplete="off" spellCheck={false} placeholder="u1…" className={`${inputClass} num`} value={seller} onChange={(e) => setSeller(e.target.value)} aria-invalid={!!errors.seller} />
          </Field>
          {format === "uniform" && (
            <Field label="Units" htmlFor="units" error={errors.units} hint="Identical items in the edition. The top N bids win and every winner pays the lowest winning bid.">
              <input id="units" name="units" type="number" min="2" max="10000" step="1" className={`${inputClass} num`} value={units} onChange={(e) => setUnits(e.target.value)} aria-invalid={!!errors.units} />
            </Field>
          )}
          <Field label="Cover image" htmlFor="image" error={errors.image} hint="Optional https URL. Without one the lot gets a generated plate.">
            <input id="image" name="image" type="url" autoComplete="off" spellCheck={false} placeholder="https://…" className={`${inputClass} num text-[13px]`} value={imageUrl} onChange={(e) => setImageUrl(e.target.value)} aria-invalid={!!errors.image} />
          </Field>
          <Field label="Open for (hours)" htmlFor="hours" error={errors.hours}>
            <input id="hours" name="hours" type="number" min="0.1" max="336" step="0.1" className={`${inputClass} num`} value={hours} onChange={(e) => setHours(e.target.value)} aria-invalid={!!errors.hours} />
          </Field>
          <Field label="Seller key" htmlFor="token" error={errors.token} hint="Your bbs_ key from the operator (or the admin token). Kept in this tab only.">
            <input id="token" name="token" type="password" autoComplete="off" className={`${inputClass} num`} value={token} onChange={(e) => setToken(e.target.value)} aria-invalid={!!errors.token} />
          </Field>
          {serverError && (
            <p className="text-sm text-danger" role="alert">
              {serverError}
            </p>
          )}
          <Button type="submit" disabled={create.isPending} className="self-start">
            {create.isPending ? "Creating…" : "Create auction"}
          </Button>
        </form>
      </Panel>
      </div>
    </div>
  );
}
