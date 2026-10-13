import Link from "next/link";
import type { ComponentProps } from "react";

const focus =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background";

export function Button({
  className = "",
  variant = "primary",
  ...props
}: ComponentProps<"button"> & { variant?: "primary" | "secondary" }) {
  const base =
    variant === "primary"
      ? "bg-accent text-accent-foreground hover:opacity-90"
      : "border border-border bg-card hover:bg-muted";
  return (
    <button
      className={`inline-flex min-h-10 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-opacity disabled:cursor-not-allowed disabled:opacity-50 ${base} ${focus} ${className}`}
      {...props}
    />
  );
}

export function LinkButton({
  className = "",
  variant = "primary",
  ...props
}: ComponentProps<typeof Link> & { variant?: "primary" | "secondary" }) {
  const base =
    variant === "primary"
      ? "bg-accent text-accent-foreground hover:opacity-90"
      : "border border-border bg-card hover:bg-muted";
  return (
    <Link
      className={`inline-flex min-h-10 items-center justify-center gap-2 rounded-md px-4 text-sm font-medium transition-opacity ${base} ${focus} ${className}`}
      {...props}
    />
  );
}

export function Card({ className = "", ...props }: ComponentProps<"div">) {
  return <div className={`rounded-lg border border-border bg-card p-5 ${className}`} {...props} />;
}

export function Skeleton({ className = "" }: { className?: string }) {
  return <div aria-hidden className={`animate-pulse rounded-md bg-muted ${className}`} />;
}

export function Mono({ className = "", ...props }: ComponentProps<"span">) {
  return <span className={`break-all font-mono text-xs ${className}`} {...props} />;
}

export function Field({
  label,
  hint,
  error,
  children,
  htmlFor,
}: {
  label: string;
  hint?: string;
  error?: string;
  htmlFor: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={htmlFor} className="text-sm font-medium">
        {label}
      </label>
      {children}
      {error ? (
        <p className="text-xs text-danger" role="alert">
          {error}
        </p>
      ) : hint ? (
        <p className="text-xs text-muted-foreground">{hint}</p>
      ) : null}
    </div>
  );
}

export const inputClass = `min-h-10 w-full rounded-md border border-border bg-background px-3 text-sm ${focus}`;

export function StatusPill({ status }: { status: string }) {
  const tone =
    status === "open"
      ? "border-success/40 text-success"
      : status === "paid" || status === "settled"
        ? "border-accent/40 text-accent"
        : "border-border text-muted-foreground";
  return <span className={`rounded-full border px-2 py-0.5 text-xs capitalize ${tone}`}>{status.replace("_", " ")}</span>;
}
