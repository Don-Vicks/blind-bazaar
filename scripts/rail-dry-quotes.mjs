#!/usr/bin/env node
// Dry quotes against both Go Dark rails, no money moved. Prints a markdown table and whether Maya will pay a
// unified address (it refuses Orchard-only ones since NU6.3). Exits 1 only if no rail answered at all.
//
//   RECIPIENT=u1… node scripts/rail-dry-quotes.mjs
//
// Endpoints and request shapes mirror apps/web/src/lib/rails/{maya,oneclick}.ts.

// Three recipients owned by one mainnet key: a lot-style Orchard-only unified address, the same key's unified
// address carrying every receiver (Orchard and transparent), and its bare transparent receiver.
const RECIPIENTS = {
  "u1 (Orchard only)": process.env.RECIPIENT_ORCHARD ?? "u1c93mnse7264d48ufvvegmmpcl9wvashlxxzad4fy03hyfyt7deqmare5cjrz60w3twaevt2s77wxc5ldqptpcyajwlvlyzn4kgtckuue",
  "u1 (Orchard + transparent)": process.env.RECIPIENT_FULL ?? "",
  "t1": process.env.RECIPIENT_T ?? "",
};
const MAYA = "https://mayanode.mayachain.info";
const ONECLICK = "https://1click.chaindefuser.com";
const USDC_ETH = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const ZEC_WANT = "1000000"; // 0.01 ZEC
// 1Click validates refundTo as a real-looking Solana account; a random 32-byte key encoded in base58 passes.
const SOL_REFUND = base58(crypto.getRandomValues(new Uint8Array(32)));
function base58(bytes) {
  const A = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
  let n = 0n;
  for (const b of bytes) n = n * 256n + BigInt(b);
  let out = "";
  while (n > 0n) { out = A[Number(n % 58n)] + out; n /= 58n; }
  for (const b of bytes) { if (b !== 0) break; out = "1" + out; }
  return out;
}

const rows = [];
let mayaOk = false;

async function maya(asset, amountIn1e8, kind, recipient) {
  const url = `${MAYA}/mayachain/quote/swap?from_asset=${encodeURIComponent(asset)}&to_asset=ZEC.ZEC&amount=${amountIn1e8}&destination=${recipient}`;
  const res = await fetch(url);
  const text = await res.text();
  let j = {};
  try { j = JSON.parse(text); } catch { /* not json */ }
  if (!res.ok || j.error) {
    rows.push(["Maya", asset.split("-")[0], kind, `refused: ${String(j.error ?? text).replace(/.*err: /, "").slice(0, 150)}`]);
    return;
  }
  rows.push(["Maya", asset.split("-")[0], kind, `accepted: ${amountIn1e8 / 1e8} in → ${(Number(j.expected_amount_out) / 1e8).toFixed(5)} ZEC out, ${j.total_swap_seconds}s, fee ${j.fees?.total_bps} bps, inbound ${String(j.inbound_address).slice(0, 12)}…`]);
  accepted++;
  if (kind.startsWith("u1")) mayaOk = true;
}

async function oneclick(originAsset, recipient, kind) {
  const body = {
    dry: true,
    swapType: "EXACT_OUTPUT",
    slippageTolerance: 100,
    originAsset,
    depositType: "ORIGIN_CHAIN",
    destinationAsset: "nep141:zec.omft.near",
    amount: ZEC_WANT,
    refundTo: SOL_REFUND,
    refundType: "ORIGIN_CHAIN",
    recipient,
    recipientType: "DESTINATION_CHAIN",
    deadline: new Date(Date.now() + 20 * 60_000).toISOString(),
    depositMode: "SIMPLE",
    referral: "godark",
  };
  const headers = { "content-type": "application/json" };
  if (process.env.ONECLICK_JWT) headers["x-api-key"] = process.env.ONECLICK_JWT;
  const res = await fetch(`${ONECLICK}/v0/quote`, { method: "POST", headers, body: JSON.stringify(body) });
  const text = await res.text();
  if (!res.ok) {
    let msg = text;
    try { msg = JSON.parse(text).message ?? text; } catch { /* keep */ }
    rows.push(["1Click", LABEL[originAsset] ?? originAsset, kind, `refused: HTTP ${res.status} ${String(msg).slice(0, 150)}`]);
    return;
  }
  const j = JSON.parse(text);
  const q = j.quote ?? {};
  rows.push(["1Click", LABEL[originAsset] ?? originAsset, kind, `accepted (dry): ${q.amountInFormatted} in → ${Number(q.amountOut) / 1e8} ZEC out, ~${q.timeEstimate}s`]);
  accepted++;
}

const LABEL = {
  "nep141:sol-5ce3bf3a31af18be40ba30f721101b4341690186.omft.near": "USDC (Solana)",
  "nep141:sol.omft.near": "SOL",
};
let accepted = 0;
for (const [kind, recipient] of Object.entries(RECIPIENTS)) {
  if (!recipient) continue;
  // A key whose "all receivers" address equals its Orchard-only one has nothing extra to test.
  if (kind.startsWith("u1 (Orchard +") && recipient === RECIPIENTS["u1 (Orchard only)"]) continue;
  await maya("ETH.ETH", 1_000_000, kind, recipient);
  await maya(`ETH.USDC-${USDC_ETH}`, 10_000_000_000, kind, recipient);
  await oneclick("nep141:sol-5ce3bf3a31af18be40ba30f721101b4341690186.omft.near", recipient, kind);
  await oneclick("nep141:sol.omft.near", recipient, kind);
}

console.log(`Dry quotes on ${new Date().toISOString().slice(0, 16)}Z, wanting ${Number(ZEC_WANT) / 1e8} ZEC\n`);
console.log("| Rail | Asset in | Recipient | Answer |\n|---|---|---|---|");
for (const r of rows) console.log(`| ${r.join(" | ")} |`);
console.log(mayaOk ? "\nMaya quotes to a unified address." : "\nMaya did not quote to any unified address: Ethereum deliveries are transparent, shielded on arrival.");
if (accepted === 0) {
  console.error("no rail answered; network or rail outage");
  process.exit(1);
}
