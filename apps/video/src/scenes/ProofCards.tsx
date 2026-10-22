import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, type } from "../theme";

const CARDS = [
  { eyebrow: "Vickrey", text: "Vickrey price proven with a Merkle path" },
  { eyebrow: "Sealed-reveal", text: "Sealed-reveal: the operator cannot read bids early" },
  { eyebrow: "Audit", text: "Audit tool: recomputes a lot from the chain, says MISMATCH when a result is forged" },
  { eyebrow: "Custody", text: "2-of-3 FROST escrow: no single server holds the money" },
];

// Four proof cards in a 2x2 grid, 14 frames apart, with the honest footnote.
export const ProofCards: React.FC = () => (
  <Backdrop seed={4}>
    <AbsoluteFill style={{ justifyContent: "center", padding: "0 180px" }}>
      <Hairline delay={0} width={260} style={{ marginBottom: 44 }} />
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 28 }}>
        {CARDS.map((c, i) => (
          <Reveal key={i} delay={10 + i * 14}>
            <div
              style={{
                backgroundColor: dark.surface,
                border: `1px solid ${dark.hairline}`,
                borderRadius: 16,
                padding: "36px 40px",
                minHeight: 240,
                display: "flex",
                flexDirection: "column",
                gap: 18,
              }}
            >
              <div style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent }}>{c.eyebrow}</div>
              <div style={{ fontFamily: fonts.sans, fontSize: type.bodySmall, lineHeight: 1.35, color: dark.fg }}>{c.text}</div>
            </div>
          </Reveal>
        ))}
      </div>
      <Reveal delay={110}>
        <div style={{ fontFamily: fonts.sans, fontSize: 30, color: dark.muted, marginTop: 40, lineHeight: 1.35 }}>
          Built and unit-tested. The FROST escrow and sealed-reveal lots have not yet signed on a chain; the audit tool has not yet run on a lot with real wallet bids.
        </div>
      </Reveal>
    </AbsoluteFill>
  </Backdrop>
);
