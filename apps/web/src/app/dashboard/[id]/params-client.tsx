"use client";

import { useParams } from "next/navigation";
import { SellerLotClient } from "@/components/seller-lot-client";

export function ParamsClient() {
  const { id } = useParams<{ id: string }>();
  return <SellerLotClient id={id} />;
}
