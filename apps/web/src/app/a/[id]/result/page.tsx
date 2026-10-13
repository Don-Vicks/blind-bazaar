import { Suspense } from "react";
import { ResultClient } from "@/components/result-client";
import { Skeleton } from "@/components/ui";

export default async function ResultPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return (
    <Suspense fallback={<Skeleton className="h-64" />}>
      <ResultClient id={id} />
    </Suspense>
  );
}
