import { Suspense } from "react";
import { ResultClient } from "@/components/result-client";
import { Skeleton } from "@/components/ui";

async function Resolved({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <ResultClient id={id} />;
}

export default function Page({ params }: { params: Promise<{ id: string }> }) {
  return (
    <Suspense fallback={<Skeleton className="h-64" />}>
      <Resolved params={params} />
    </Suspense>
  );
}
