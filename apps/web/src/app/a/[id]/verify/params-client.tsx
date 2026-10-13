"use client";

import { useParams } from "next/navigation";
import { VerifyClient } from "@/components/verify-client";

export function ParamsClient() {
  const { id } = useParams<{ id: string }>();
  return <VerifyClient id={id} />;
}
