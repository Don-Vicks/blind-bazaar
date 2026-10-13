"use client";

import { GoDark, type GoDarkOptions } from "@blind-bazaar/go-dark";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/lib/api";

export function GoDarkDemo(props: Partial<GoDarkOptions> & { amountZats: string }) {
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  // The widget reads window at mount; render it only after hydration so server and client HTML agree.
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  const to = props.to ?? health.data?.operator_address;
  if (!mounted || !to) return <p className="text-[13px] text-muted-foreground">Waiting for the service to publish a receive address…</p>;
  return <GoDark {...props} to={to} amountZats={props.amountZats} />;
}
