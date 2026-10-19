"use client";

import { useMutation } from "@tanstack/react-query";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Suspense } from "react";
import { api } from "@/lib/api";
import { Button, Eyebrow, Panel } from "@/components/ui";

function Confirm() {
  const token = useSearchParams().get("token") ?? "";
  const confirm = useMutation({ mutationFn: () => api.confirmNotify(token) });
  return (
    <Panel className="mx-auto max-w-[560px]">
      <Eyebrow>Email updates</Eyebrow>
      {confirm.isSuccess ? (
        <>
          <p className="serif mt-2 text-[26px] leading-tight">Confirmed.</p>
          <p className="mt-3 text-[14px] text-muted-foreground">You will be emailed when something needs you: a bid received, a reveal window, a result, a refund or a payout. Addresses are removed a week after a lot ends.</p>
          <Link href="/bids" className="mt-4 inline-block text-[14px] underline">Your bids</Link>
        </>
      ) : (
        <>
          <p className="serif mt-2 text-[26px] leading-tight">Turn on email updates?</p>
          <p className="mt-3 text-[14px] text-muted-foreground">Someone asked Blind Bazaar to email this address. Confirm once and nothing else is needed. If you did not ask, close this page; no mail will follow.</p>
          <div className="mt-4 flex items-center gap-3">
            <Button onClick={() => confirm.mutate()} disabled={!token || confirm.isPending}>{confirm.isPending ? "Confirming…" : "Yes, email me"}</Button>
            {confirm.isError && <span className="text-[13px] text-danger">{(confirm.error as Error).message}</span>}
            {!token && <span className="text-[13px] text-danger">This link is missing its token.</span>}
          </div>
        </>
      )}
    </Panel>
  );
}

export default function ConfirmPage() {
  return (
    <Suspense fallback={null}>
      <Confirm />
    </Suspense>
  );
}
