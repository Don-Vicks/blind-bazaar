import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Caption, CaptionItem } from "../components/Caption";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, TX, type } from "../theme";

// From docs/evidence/testnet-jck1vba7/txids.md, rows 4 to 6.
const STEPS = [
  { n: "04", title: "The winner pays the 0.02 balance", body: "settlement by memo, matched to the winning bid's nonce", txid: TX.settlement },
  { n: "05", title: "The seller is paid 0.03", body: "payout to the seller's own wallet", txid: TX.payout },
  { n: "06", title: "The loser is refunded 0.01", body: "after the random delay, to the address sealed in the losing memo", txid: TX.refund },
];

// After the close: three shielded transactions on testnet. These pages are operator-only, so a composed card.
export const SettlementCard: React.FC<{ captions: CaptionItem[] }> = ({ captions }) => (
  <Backdrop seed={1} tint={0.7}>
    <AbsoluteFill style={{ padding: "84px 200px 0" }}>
      <Reveal delay={0}>
        <div style={{ fontFamily: fonts.serif, fontSize: type.h3, color: dark.fg, lineHeight: 1.1 }}>
          After the close <span style={{ fontStyle: "italic", color: dark.accent }}>on testnet</span>
        </div>
      </Reveal>
      <Hairline delay={10} width={260} style={{ margin: "26px 0 30px" }} />
      {STEPS.map((s, i) => (
        <Reveal key={i} delay={20 + i * 16} style={{ display: "flex", gap: 40, alignItems: "baseline", marginBottom: 30, maxWidth: 1520 }}>
          <span style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent, width: 56, flexShrink: 0 }}>{s.n}</span>
          <div>
            <div style={{ fontFamily: fonts.serif, fontSize: 46, color: dark.fg, lineHeight: 1.15 }}>{s.title}</div>
            <div style={{ fontFamily: fonts.sans, fontSize: 32, color: dark.muted, lineHeight: 1.35, marginTop: 6 }}>{s.body}</div>
            <div style={{ fontFamily: fonts.mono, fontSize: 28, color: dark.fg, opacity: 0.8, marginTop: 8, letterSpacing: 0.3 }}>{s.txid}</div>
          </div>
        </Reveal>
      ))}
    </AbsoluteFill>
    <Caption items={captions} />
  </Backdrop>
);
