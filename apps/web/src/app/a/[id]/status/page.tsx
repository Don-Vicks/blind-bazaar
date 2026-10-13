import { Suspense } from "react";
import { StatusClient } from "@/components/status-client";
import { Skeleton } from "@/components/ui";

export default async function StatusPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return (
    <Suspense fallback={<Skeleton className="h-64" />}>
      <StatusClient id={id} />
    </Suspense>
  );
}
