"use client";

import { GoDark, type GoDarkOptions } from "@blind-bazaar/go-dark";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function GoDarkDemo(props: Partial<GoDarkOptions> & { amountZats: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const to = props.to ?? health.data?.operator_address;
  if (!to) return <p className="text-sm text-muted-foreground">Waiting for the service to publish a receive address…</p>;
  return <GoDark {...props} to={to} amountZats={props.amountZats} />;
}
