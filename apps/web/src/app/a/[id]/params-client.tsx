"use client";

import { useParams } from "next/navigation";
import { AuctionClient } from "@/components/auction-client";

export function ParamsClient() {
  const { id } = useParams<{ id: string }>();
  return <AuctionClient id={id} />;
}
