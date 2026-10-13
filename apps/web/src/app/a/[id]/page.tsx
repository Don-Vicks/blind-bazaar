import { Suspense } from "react";
import { AuctionClient } from "@/components/auction-client";
import { Skeleton } from "@/components/ui";

export default async function AuctionPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return (
    <Suspense fallback={<Skeleton className="h-64" />}>
      <AuctionClient id={id} />
    </Suspense>
  );
}
