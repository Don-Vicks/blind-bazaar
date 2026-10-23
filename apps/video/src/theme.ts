import { loadFont as loadSerif } from "@remotion/google-fonts/InstrumentSerif";
import { loadFont as loadSans } from "@remotion/google-fonts/Geist";
import { loadFont as loadMono } from "@remotion/google-fonts/GeistMono";

// Fonts: the site's own pairing. Instrument Serif for headlines (italic for emphasis),
// Geist for body, Geist Mono for txids and commands.
const serif = loadSerif("normal", { weights: ["400"], subsets: ["latin"] });
loadSerif("italic", { weights: ["400"], subsets: ["latin"] });
const sans = loadSans("normal", { weights: ["400", "500"], subsets: ["latin"] });
const mono = loadMono("normal", { weights: ["400"], subsets: ["latin"] });

export const fonts = {
  serif: serif.fontFamily,
  sans: sans.fontFamily,
  mono: mono.fontFamily,
};

// Colors from apps/web/src/app/globals.css.
export const dark = {
  bg: "#1b1511",
  surface: "#231b16",
  fg: "#f1e7d8",
  muted: "#a69583",
  accent: "#d99a5b",
  hairline: "rgba(241,231,216,0.11)",
  hairlineStrong: "rgba(241,231,216,0.3)",
  success: "#9cc49f",
  danger: "#e3826f",
};

export const light = {
  bg: "#f3ebdf",
  surface: "#faf4ea",
  fg: "#231b14",
  muted: "#6e6153",
  accent: "#a8542c",
  hairline: "rgba(35,27,20,0.12)",
};

// The lot-plate inks (apps/web/src/components/lot-plate.tsx).
export const inks = ["#b7542f", "#c99a4a", "#7f8b5a", "#5b5f8a", "#a0463a", "#d9b27a", "#6f5a3c"];

// One spring for everything: no bounce.
export const SPRING = { damping: 200 } as const;
export const SPRING_SLOW = { damping: 200, stiffness: 60 } as const;

// Type scale (px). Nothing below 28.
export const type = {
  display: 96,
  headline: 84,
  h2: 64,
  h3: 56,
  body: 40,
  bodySmall: 36,
  caption: 34,
  label: 28,
};

export const site = process.env.REMOTION_SITE_URL ?? "http://localhost:3001";
export const useIframe = process.env.REMOTION_NO_IFRAME !== "1";

export const LOT = "08t6ynq2";

// From docs/evidence/testnet-08t6ynq2/txids.md (the public tender).
export const TX = {
  bidA: "9fdd59ef7e32aedca48a0d987046bfa5938c9428f254a086a9bee6d96c0b7c30", // winning quote, 0.025
  bidB: "532390f3b988b1e1d11fa7323a118a0cc025654936a5927d95cdf386db4a05f7", // losing quote, 0.03
  settlement: "3632a5ccb1fcf77a89f3cdff6302bb963170c2ae6302827fe448e1d8fc4f4817", // the buyer pays the winning quote
  payout: "64a30a9ecbb12e08a81f96b50909edfeb7471a42c51ac9040d55d47d7ce93a39", // supplier: quote plus bond
  refund: "58186ba83646c2462027c1bf93b3b85cd666ea9a29358d660c77a799338e7e2f", // losing supplier's bond
};
export const CLOSE_BLOCK = "4,532,675";

export const REPO = "github.com/Don-Vicks/blind-bazaar";
