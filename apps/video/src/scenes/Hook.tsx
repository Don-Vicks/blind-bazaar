import React from "react";
import { AbsoluteFill, useVideoConfig } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, type } from "../theme";

// Title card: the claim against Ethereum, then what this one does instead.
export const Hook: React.FC = () => {
  const { width, height } = useVideoConfig();
  const vertical = height > width;
  const size = vertical ? 76 : type.headline;
  const pad = vertical ? 72 : 160;
  return (
    <Backdrop seed={0}>
      <AbsoluteFill style={{ justifyContent: "center", padding: `0 ${pad}px` }}>
        <div style={{ maxWidth: vertical ? 940 : 1640 }}>
          <Reveal delay={6}>
            <div style={{ fontFamily: fonts.serif, fontSize: size, lineHeight: 1.12, color: dark.fg }}>
              Every auction on Ethereum publishes every bid forever.
            </div>
          </Reveal>
          <Hairline delay={20} width={vertical ? 200 : 260} style={{ margin: vertical ? "40px 0" : "44px 0" }} />
          <Reveal delay={32}>
            <div style={{ fontFamily: fonts.serif, fontStyle: "italic", fontSize: size, lineHeight: 1.12, color: dark.accent }}>
              This one publishes the winner, and a proof.
            </div>
          </Reveal>
        </div>
      </AbsoluteFill>
    </Backdrop>
  );
};
