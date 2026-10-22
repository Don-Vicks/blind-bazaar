import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Caption, CaptionItem } from "../components/Caption";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, type } from "../theme";

const STEPS = [
  { n: "01", title: "The loser's deposit comes back", body: "A shielded payment to the refund address in the memo, after a random delay. Single refunds ran on regtest; batching is unit-tested." },
  { n: "02", title: "The winner pays the balance", body: "A second payment to the lot, by memo. From the bundled wallet it is one command and one block." },
  { n: "03", title: "The seller is paid", body: "On the chain all three are ordinary shielded transactions." },
];

// After close: refund, settlement, payout, as a composed card (these pages are operator-only).
export const SettlementCard: React.FC<{ captions: CaptionItem[] }> = ({ captions }) => (
  <Backdrop seed={1} tint={0.7}>
    <AbsoluteFill style={{ padding: "110px 200px 0" }}>
      <Reveal delay={0}>
        <div style={{ fontFamily: fonts.serif, fontSize: type.h2, color: dark.fg, lineHeight: 1.1 }}>After the close</div>
      </Reveal>
      <Hairline delay={10} width={260} style={{ margin: "32px 0 40px" }} />
      {STEPS.map((s, i) => (
        <Reveal key={i} delay={20 + i * 16} style={{ display: "flex", gap: 40, alignItems: "baseline", marginBottom: 40, maxWidth: 1400 }}>
          <span style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent, width: 56, flexShrink: 0 }}>{s.n}</span>
          <div>
            <div style={{ fontFamily: fonts.serif, fontSize: 48, color: dark.fg, lineHeight: 1.15 }}>{s.title}</div>
            <div style={{ fontFamily: fonts.sans, fontSize: type.bodySmall, color: dark.muted, lineHeight: 1.4, marginTop: 10 }}>{s.body}</div>
          </div>
        </Reveal>
      ))}
    </AbsoluteFill>
    <Caption items={captions} />
  </Backdrop>
);
