"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function HealthBadge() {
  const { data, isError } = useQuery({ queryKey: ["health"], queryFn: api.health, refetchInterval: 15_000 });
  const lag = data?.sync_lag ?? null;
  const ok = data && !data.last_error && lag !== null && lag <= 2;
  const label = isError
    ? "offline"
    : !data
      ? "connecting"
      : data.low_balance
        ? "low balance"
        : ok
          ? `${data.network === "main" ? "mainnet" : data.network} · synced`
          : `syncing · ${lag ?? "?"} behind`;
  const custody = data?.signing?.startsWith("frost") ? data.signing.split(" (")[0] : null;
  return (
    <span className="inline-flex items-center gap-2 text-[12px] text-muted-foreground" aria-live="polite">
      <span
        aria-hidden
        className={`size-1.5 rounded-full ${isError || data?.low_balance ? "bg-danger" : ok ? "bg-accent" : "bg-muted-foreground/50"}`}
      />
      {label}
      {custody ? <span className="text-muted-foreground/70" title={data?.signing ?? undefined}>· {custody}</span> : null}
    </span>
  );
}
