import React from "react";
import { AbsoluteFill, useVideoConfig } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, REPO, type } from "../theme";

// End card: eyebrow, name, hairline, repo. Everything is in by frame 60 and holds.
export const EndCard: React.FC = () => {
  const { width, height } = useVideoConfig();
  const vertical = height > width;
  return (
    <Backdrop seed={6}>
      <AbsoluteFill style={{ justifyContent: "center", alignItems: "center", padding: vertical ? "0 72px" : "0 160px" }}>
        <Reveal delay={0}>
          <div style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent, letterSpacing: 2, textAlign: "center" }}>
            ZECATHON 2026 · Private markets
          </div>
        </Reveal>
        <Reveal delay={12} style={{ marginTop: 36 }}>
          <div style={{ fontFamily: fonts.serif, fontSize: vertical ? 84 : type.display, lineHeight: 1.08, color: dark.fg, textAlign: "center" }}>
            Blind Bazaar <span style={{ color: dark.muted }}>+</span> Go Dark
          </div>
        </Reveal>
        <Hairline delay={26} width={260} align="center" style={{ margin: "44px 0" }} />
        <Reveal delay={38}>
          <div style={{ fontFamily: fonts.mono, fontSize: vertical ? 30 : type.caption, color: dark.fg, textAlign: "center" }}>{REPO}</div>
        </Reveal>
      </AbsoluteFill>
    </Backdrop>
  );
};
