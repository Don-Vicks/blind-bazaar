import Link from "next/link";
import type { ComponentProps } from "react";

export const focus =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background";

const btnBase = `inline-flex min-h-11 items-center justify-center gap-2 rounded-[2px] px-5 text-[13px] font-medium tracking-[0.02em] transition-[background-color,color,border-color,opacity] duration-150 ease-out disabled:cursor-not-allowed disabled:opacity-40 ${focus}`;
const btnVariants = {
  primary: "bg-foreground text-background hover:opacity-90",
  secondary: "border border-hairline-strong text-foreground hover:bg-foreground/5",
  ghost: "text-muted-foreground hover:text-foreground",
} as const;

export function Button({
  className = "",
  variant = "primary",
  ...props
}: ComponentProps<"button"> & { variant?: keyof typeof btnVariants }) {
  return <button className={`${btnBase} ${btnVariants[variant]} ${className}`} {...props} />;
}

export function LinkButton({
  className = "",
  variant = "primary",
  ...props
}: ComponentProps<typeof Link> & { variant?: keyof typeof btnVariants }) {
  return <Link className={`${btnBase} ${btnVariants[variant]} ${className}`} {...props} />;
}

/** Raised panel: the only "card" in the system. Used for the bid panel and result, nothing else. */
export function Panel({ className = "", ...props }: ComponentProps<"section">) {
  return <section className={`rounded-md border border-hairline bg-surface p-6 sm:p-8 ${className}`} {...props} />;
}

export function Skeleton({ className = "" }: { className?: string }) {
  return <div aria-hidden className={`animate-pulse rounded-[2px] bg-foreground/5 ${className}`} />;
}

export function Mono({ className = "", ...props }: ComponentProps<"span">) {
  return <span className={`num break-all text-[13px] ${className}`} {...props} />;
}

export function Eyebrow({ className = "", ...props }: ComponentProps<"p">) {
  return <p className={`eyebrow ${className}`} {...props} />;
}

/** Label + value pair used for facts. Value in mono, label as eyebrow. */
export function Fact({ label, children, large = false, hint }: { label: string; children: React.ReactNode; large?: boolean; hint?: string }) {
  return (
    <div className="flex flex-col gap-1.5">
      <Eyebrow>{label}</Eyebrow>
      <div className={large ? "serif text-[28px] leading-none" : "num text-[15px]"}>{children}</div>
      {hint && <p className="text-[12px] leading-snug text-muted-foreground">{hint}</p>}
    </div>
  );
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
    <div className="flex flex-col gap-2">
      <label htmlFor={htmlFor} className="eyebrow text-foreground">
        {label}
      </label>
      {children}
      {error ? (
        <p className="text-[13px] text-danger" role="alert">
          {error}
        </p>
      ) : hint ? (
        <p className="text-[13px] text-muted-foreground">{hint}</p>
      ) : null}
    </div>
  );
}

export const inputClass = `min-h-11 w-full rounded-[2px] border border-hairline-strong bg-transparent px-3 text-[15px] placeholder:text-muted-foreground/60 transition-colors duration-150 hover:border-foreground/40 focus:border-foreground ${focus}`;

export function StatusDot({ status }: { status: string }) {
  const live = status === "open" || status === "revealing";
  const done = status === "paid" || status === "settled";
  return (
    <span className="inline-flex items-center gap-2 text-[13px] text-muted-foreground">
      <span
        aria-hidden
        className={`size-1.5 rounded-full ${live ? "bg-accent" : done ? "bg-success" : "bg-muted-foreground/60"}`}
      />
      <span className="capitalize">{status.replace("_", " ")}</span>
    </span>
  );
}
