"use client";

import { looksLikeShieldedAddress, parseZec } from "@blind-bazaar/sdk";
import { useMutation } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { api, ApiError } from "@/lib/api";
import { Button, Eyebrow, Field, Panel, inputClass } from "@/components/ui";

const TOKEN_KEY = "bb.adminToken";

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
  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    try {
      setToken(sessionStorage.getItem(TOKEN_KEY) ?? "");
    } catch {
      /* no storage */
    }
  }, []);

  const create = useMutation({
    mutationFn: (body: Record<string, unknown>) => api.createAuction(token, body),
    onSuccess: (a) => router.push(`/a/${a.id}`),
  });

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    const next: Record<string, string> = {};
    let reserveZats = 0n;
    let depositZats = 0n;
    if (!token.trim()) next.token = "Enter the operator's admin token.";
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
    const u = Number(units);
    if (!(Number.isInteger(u) && u >= 1 && u <= 10000)) next.units = "A whole number from 1 to 10000.";
    setErrors(next);
    if (Object.keys(next).length) return;
    try {
      sessionStorage.setItem(TOKEN_KEY, token.trim());
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
      units: Number(units),
    });
  }

  const serverError =
    create.error instanceof ApiError
      ? create.error.status === 401
        ? "The admin token was rejected."
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
          <div><dt className="text-foreground">Reserve</dt><dd className="text-muted-foreground">The lowest bid you will accept. Published.</dd></div>
          <div><dt className="text-foreground">Deposit</dt><dd className="text-muted-foreground">Paid by every bidder to bid. Returned to losers, credited to the winner.</dd></div>
          <div><dt className="text-foreground">Editions</dt><dd className="text-muted-foreground">Set units above 1 for identical items. Uniform price: everyone who wins pays the same, the lowest winning bid.</dd></div>
          <div><dt className="text-foreground">Payout address</dt><dd className="text-muted-foreground">Your unified address. The winning amount arrives shielded.</dd></div>
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
          <div className="grid gap-5 sm:grid-cols-2">
            <Field label="Reserve (ZEC)" htmlFor="reserve" error={errors.reserve} hint="Lowest bid you will accept.">
              <input id="reserve" name="reserve" type="text" inputMode="decimal" autoComplete="off" placeholder="0.5" className={`${inputClass} num`} value={reserve} onChange={(e) => setReserve(e.target.value)} aria-invalid={!!errors.reserve} />
            </Field>
            <Field label="Deposit (ZEC)" htmlFor="deposit" error={errors.deposit} hint="Every bidder pays this to bid. Losers get it back.">
              <input id="deposit" name="deposit" type="text" inputMode="decimal" autoComplete="off" placeholder="0.05" className={`${inputClass} num`} value={deposit} onChange={(e) => setDeposit(e.target.value)} aria-invalid={!!errors.deposit} />
            </Field>
          </div>
          <Field label="Payout address" htmlFor="seller" error={errors.seller} hint="Your unified address. The winning amount is sent here shielded.">
            <input id="seller" name="seller" type="text" autoComplete="off" spellCheck={false} placeholder="u1…" className={`${inputClass} num`} value={seller} onChange={(e) => setSeller(e.target.value)} aria-invalid={!!errors.seller} />
          </Field>
          <Field label="Units" htmlFor="units" error={errors.units} hint="1 for a single lot. More than 1 makes it an edition: the top N bids win and every winner pays the lowest winning bid.">
            <input id="units" name="units" type="number" min="1" max="10000" step="1" className={`${inputClass} num`} value={units} onChange={(e) => setUnits(e.target.value)} aria-invalid={!!errors.units} />
          </Field>
          <Field label="Cover image" htmlFor="image" error={errors.image} hint="Optional https URL. Without one the lot gets a generated plate.">
            <input id="image" name="image" type="url" autoComplete="off" spellCheck={false} placeholder="https://…" className={`${inputClass} num text-[13px]`} value={imageUrl} onChange={(e) => setImageUrl(e.target.value)} aria-invalid={!!errors.image} />
          </Field>
          <Field label="Open for (hours)" htmlFor="hours" error={errors.hours}>
            <input id="hours" name="hours" type="number" min="0.1" max="336" step="0.1" className={`${inputClass} num`} value={hours} onChange={(e) => setHours(e.target.value)} aria-invalid={!!errors.hours} />
          </Field>
          <Field label="Admin token" htmlFor="token" error={errors.token} hint="Set by the operator running the service. Kept in this tab only.">
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
