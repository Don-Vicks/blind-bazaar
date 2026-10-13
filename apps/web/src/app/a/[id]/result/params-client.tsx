"use client";

import { useParams } from "next/navigation";
import { ResultClient } from "@/components/result-client";

export function ParamsClient() {
  const { id } = useParams<{ id: string }>();
  return <ResultClient id={id} />;
}
