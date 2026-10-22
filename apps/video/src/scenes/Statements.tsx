import React from "react";
import { AbsoluteFill, useVideoConfig } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, type } from "../theme";

const LINES = [
  "A bid is one shielded Zcash payment.",
  "The amount travels inside the encrypted memo.",
  "Nobody sees the losers. Not even the seller.",
];

// Three statements, one at a time, each given room to be read.
export const Statements: React.FC = () => {
  const { width, height } = useVideoConfig();
  const vertical = height > width;
  const size = vertical ? 60 : type.h2;
  const pad = vertical ? 72 : 200;
  const gap = vertical ? 44 : 52;
  return (
    <Backdrop seed={1}>
      <AbsoluteFill style={{ justifyContent: "center", padding: `0 ${pad}px` }}>
        <Hairline delay={0} width={vertical ? 200 : 260} style={{ marginBottom: 56 }} />
        {LINES.map((line, i) => (
          <Reveal key={i} delay={10 + i * 38} style={{ marginBottom: gap, display: "flex", gap: vertical ? 24 : 40, alignItems: "baseline" }}>
            <span style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent, width: 56, flexShrink: 0 }}>
              0{i + 1}
            </span>
            <span style={{ fontFamily: fonts.serif, fontSize: size, lineHeight: 1.15, color: dark.fg }}>{line}</span>
          </Reveal>
        ))}
      </AbsoluteFill>
    </Backdrop>
  );
};
