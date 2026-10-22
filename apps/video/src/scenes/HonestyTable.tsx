import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Caption, CaptionItem } from "../components/Caption";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, type } from "../theme";

// The first five rows of the README honesty table, each claim under eight words.
const ROWS: { claim: string; tag: "REAL" | "NOT YET"; note: string }[] = [
  { claim: "Bids are private; losers never revealed", tag: "REAL", note: "operator reads classic lots while open; sealed-reveal removes that" },
  { claim: "No wallet? Make one in the browser", tag: "REAL", note: "24 words and a unified address, client-side; nothing stored or sent" },
  { claim: "Bid by QR from a consumer wallet", tag: "NOT YET", note: "only the bundled command-line wallet has placed a bid" },
  { claim: "Result proven by a note-opening disclosure", tag: "REAL", note: "regtest and the testnet service; no mainnet lot yet" },
  { claim: "Ledger root fixed before any refund", tag: "REAL", note: "Merkle root over every valid bid; each bidder checks their leaf" },
];

export const HonestyTable: React.FC<{ captions: CaptionItem[] }> = ({ captions }) => (
  <Backdrop seed={6} tint={0.7}>
    <AbsoluteFill style={{ padding: "92px 200px 0" }}>
      <Reveal delay={0}>
        <div style={{ fontFamily: fonts.serif, fontSize: type.h3, color: dark.fg, lineHeight: 1.1 }}>
          What is private, what is real, <span style={{ fontStyle: "italic", color: dark.accent }}>what is not</span>
        </div>
      </Reveal>
      <Hairline delay={8} width="100%" style={{ margin: "30px 0 0" }} />
      {ROWS.map((r, i) => (
        <Reveal key={i} delay={18 + i * 14}>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "640px 160px 1fr",
              gap: 40,
              alignItems: "baseline",
              padding: "24px 0",
              borderBottom: `1px solid ${dark.hairline}`,
            }}
          >
            <div style={{ fontFamily: fonts.sans, fontSize: type.bodySmall, color: dark.fg, lineHeight: 1.3 }}>{r.claim}</div>
            <div style={{ fontFamily: fonts.mono, fontSize: type.label, color: r.tag === "REAL" ? dark.success : dark.accent, letterSpacing: 1 }}>{r.tag}</div>
            <div style={{ fontFamily: fonts.sans, fontSize: 30, color: dark.muted, lineHeight: 1.35 }}>{r.note}</div>
          </div>
        </Reveal>
      ))}
    </AbsoluteFill>
    <Caption items={captions} />
  </Backdrop>
);
