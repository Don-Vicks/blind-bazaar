import { mayaQuote, mayaStatus } from "./maya";
import { oneclickQuote, oneclickStatus, oneclickSubmit } from "./oneclick";
import type { Quote, QuoteRequest, Rail, StatusResponse } from "./types";

export * from "./types";

export function isAcceptableZcashAddress(a: string): boolean {
  // Rails validate u1/t1 today; zu/tu/zs are rejected client-side to avoid lost funds.
  return /^(u1|t1|t3)[0-9a-zA-Z]{20,}$/.test(a);
}

export function pickRail(req: QuoteRequest): Rail {
  // Maya delivers to unified addresses from Ethereum with no API key; 1Click is the only Solana rail.
  return req.chain === "ethereum" ? "maya" : "oneclick";
}

export async function getQuote(req: QuoteRequest): Promise<Quote> {
  return pickRail(req) === "maya" ? mayaQuote(req) : oneclickQuote(req);
}

export async function submitDeposit(rail: Rail, depositAddress: string, txHash: string): Promise<void> {
  if (rail === "oneclick") await oneclickSubmit(depositAddress, txHash);
}

export async function getStatus(rail: Rail, id: string): Promise<StatusResponse> {
  return rail === "maya" ? mayaStatus(id) : oneclickStatus(id);
}
