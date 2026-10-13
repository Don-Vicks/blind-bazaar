"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function HealthBadge() {
  const { data, isError } = useQuery({ queryKey: ["health"], queryFn: api.health, refetchInterval: 15_000 });
  const lag = data?.sync_lag ?? null;
  const ok = data && !data.last_error && lag !== null && lag <= 2;
  const label = isError ? "service offline" : !data ? "connecting" : data.low_balance ? "low balance" : ok ? "synced" : `syncing (${lag ?? "?"} behind)`;
  return (
    <span className="inline-flex items-center gap-2 rounded-full border border-border px-2.5 py-1 text-xs" aria-live="polite">
      <span
        aria-hidden
        className={`size-2 rounded-full ${isError ? "bg-danger" : data?.low_balance ? "bg-danger" : ok ? "bg-success" : "bg-accent"}`}
      />
      {data?.network === "test" ? "testnet · " : data?.network === "main" ? "mainnet · " : ""}
      {label}
    </span>
  );
}
