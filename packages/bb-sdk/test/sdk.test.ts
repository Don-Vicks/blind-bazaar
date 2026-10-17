import { describe, expect, it } from "vitest";
import {
  buildPaymentUri,
  encodeBid,
  formatZec,
  ledgerHash,
  memoFromBase64Url,
  memoToBase64Url,
  parseMemo,
  parsePaymentUri,
  parseZec,
  prepareBid,
  nonceFromSecret,
  receipt,
  leafHash,
  verifyInclusion,
} from "../src";

const UA =
  "utest1zkkkjfxkamagznjr6ayemffj2d2gacdwpzcyw669pvg06xevzqslpmm27zjsctlkstl2vsw62xrjktmzqcu4yu9zdhdxqz3kafa4j2q85y6mv74rzjcgjg8c0ytrg7dwyzwtgnuc76h";

describe("memo", () => {
  it("round-trips a bid", () => {
    const bid = { auctionId: "7k2m9q4e", bidZats: 150_000_000n, refundAddress: UA, nonce: "9f3a0c7b12de44aa" };
    expect(parseMemo(encodeBid(bid))).toEqual({ kind: "bid", bid });
  });
  it("rejects garbage", () => {
    expect(() => parseMemo("hello")).toThrow();
    expect(() => parseMemo(`BB1|7K2M9Q4E|1|${UA}|9f3a0c7b12de44aa`)).toThrow();
    expect(() => parseMemo(`BB1|7k2m9q4e|1|t1abc|9f3a0c7b12de44aa`)).toThrow();
  });
  it("receipt is sha256 hex", async () => {
    expect(await receipt("9f3a0c7b12de44aa")).toMatch(/^[0-9a-f]{64}$/);
  });
  it("ledger hash is order independent", async () => {
    expect(await ledgerHash([["r1", 5n], ["r2", 7n]])).toBe(await ledgerHash([["r2", 7n], ["r1", 5n]]));
  });
});

describe("zip321", () => {
  it("formats ZEC per spec", () => {
    expect(formatZec(100_000_000n)).toBe("1");
    expect(formatZec(1_000_000n)).toBe("0.01");
    expect(formatZec(123_456_789n)).toBe("1.23456789");
    expect(parseZec("0.01")).toBe(1_000_000n);
  });
  it("base64url has no padding or +/", () => {
    const enc = memoToBase64Url("BB1|7k2m9q4e|1|x|y???");
    expect(enc).not.toMatch(/[+/=]/);
    expect(memoFromBase64Url(enc)).toBe("BB1|7k2m9q4e|1|x|y???");
  });
  it("builds and parses a URI", () => {
    const uri = buildPaymentUri({ address: UA, amountZats: 1_000_000n, memo: "BB1|x" });
    expect(uri.startsWith(`zcash:${UA}?amount=0.01&memo=`)).toBe(true);
    expect(parsePaymentUri(uri)).toEqual({ address: UA, amountZats: 1_000_000n, memo: "BB1|x" });
  });
  it("refuses memo on transparent address", () => {
    expect(() => buildPaymentUri({ address: "t1abc", amountZats: 1n, memo: "x" })).toThrow();
  });
});

describe("prepareBid", () => {
  it("produces a consistent bundle", async () => {
    const p = await prepareBid({ id: "7k2m9q4e", address: UA, deposit_zats: 1_000_000 }, 5_000_000n, UA);
    expect(p.memo.startsWith("BB1|7k2m9q4e|5000000|")).toBe(true);
    expect(parsePaymentUri(p.uri).memo).toBe(p.memo);
    expect(p.receipt).toBe(await receipt(p.bid.nonce));
    expect(p.secret).toMatch(/^[0-9a-f]{32}$/);
    expect(p.bid.nonce).toBe(await nonceFromSecret(p.secret));
    expect(p.memo).not.toContain(p.secret);
  });
});

describe("merkle", () => {
  it("leaf hash matches the Rust layout", async () => {
    // Pinned from `cargo test -p bb-auction vectors -- --nocapture`: sha256(0x00 || "ab" || 0x0000000000000001)
    expect(await leafHash("ab", 1n, "00000000000000ab")).toBe("2bf19de1cea5e8da7d84ecfc5be99e8d60b292bc9493df50effc24e62c924205");
  });
  it("verifies a single-leaf proof", async () => {
    const leaf = await leafHash("r1", 5n, "0000000000000001");
    expect(await verifyInclusion("r1", 5n, "0000000000000001", { leaf, index: 0, leaf_count: 1, path: [], root: leaf })).toBe(true);
    expect(await verifyInclusion("r1", 6n, "0000000000000001", { leaf, index: 0, leaf_count: 1, path: [], root: leaf })).toBe(false);
    expect(await verifyInclusion("r1", 5n, "0000000000000002", { leaf, index: 0, leaf_count: 1, path: [], root: leaf })).toBe(false);
  });
});
