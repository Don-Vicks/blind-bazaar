"use client";

import { useParams } from "next/navigation";
import { StatusClient } from "@/components/status-client";

export function ParamsClient() {
  const { id } = useParams<{ id: string }>();
  return <StatusClient id={id} />;
}
