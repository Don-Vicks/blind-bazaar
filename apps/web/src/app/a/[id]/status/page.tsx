import { Suspense } from "react";
import { Skeleton } from "@/components/ui";
import { ParamsClient } from "./params-client";

export default function Page() {
  return (
    <Suspense fallback={<Skeleton className="h-64" />}>
      <ParamsClient />
    </Suspense>
  );
}
