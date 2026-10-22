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

export const LOT = "jck1vba7";

// From docs/evidence/testnet-jck1vba7/txids.md.
export const TX = {
  bidA: "e3a1be8fcab994d6ac90c5e239d78cac19c7c8d532e03b6bd1e8bb4005ee1557",
  bidB: "60af4ab5863d5b5d7e49402fd9fe6f957b1afad33d8a3e0bb3c895dd07b13ba6",
  settlement: "f258e33378d2396dcedaf9d81fa23636ca1a0fa5de0bd56067505eaf153030b1",
  payout: "9c4e120d6a122d1add32454293c56bb991f5df467f4312263cc5bce7a27950b1",
  refund: "0e2f2de7b397568c3e918ed9e503ae3b1a7e15dec2f4408897e652bb6faa99d6",
};
export const CLOSE_BLOCK = "4,530,003";

export const REPO = "github.com/Don-Vicks/blind-bazaar";
