import { formatZec as sdkFormatZec } from "@blind-bazaar/sdk";

export function zec(zats: number | bigint | null | undefined, ticker = "ZEC"): string {
  if (zats === null || zats === undefined) return "—";
  return `${sdkFormatZec(BigInt(zats))} ${ticker}`;
}

export function shortAddress(a: string, head = 10, tail = 6): string {
  return a.length <= head + tail + 1 ? a : `${a.slice(0, head)}…${a.slice(-tail)}`;
}

export function shortTxid(t: string): string {
  return `${t.slice(0, 8)}…${t.slice(-8)}`;
}

export function countdown(seconds: number): string {
  if (seconds <= 0) return "closed";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function dateTime(unix: number): string {
  return new Date(unix * 1000).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}
