import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, TX, type } from "../theme";

// "The chain closes it, not our clock." and the bid A txid typing out by string slicing.
export const ChainCloses: React.FC = () => {
  const frame = useCurrentFrame();
  const { width, height } = useVideoConfig();
  const vertical = height > width;
  const start = 34;
  const chars = Math.floor(interpolate(frame, [start, start + 54], [0, TX.bidA.length], { extrapolateLeft: "clamp", extrapolateRight: "clamp" }));
  const typed = TX.bidA.slice(0, chars);
  const done = chars >= TX.bidA.length;
  const cursorOn = !done && frame % 24 < 14;
  const pad = vertical ? 72 : 200;
  const monoSize = vertical ? 32 : 38;
  return (
    <Backdrop seed={3}>
      <AbsoluteFill style={{ justifyContent: "center", padding: `0 ${pad}px` }}>
        <Reveal delay={0}>
          <div style={{ fontFamily: fonts.serif, fontSize: vertical ? 68 : 76, lineHeight: 1.12, color: dark.fg, maxWidth: 1300 }}>
            The chain closes it, not our clock.
          </div>
        </Reveal>
        <Hairline delay={14} width={vertical ? 200 : 260} style={{ margin: vertical ? "40px 0" : "48px 0" }} />
        <Reveal delay={24}>
          <div style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent, marginBottom: 18 }}>bid A · txid</div>
        </Reveal>
        <div
          style={{
            fontFamily: fonts.mono,
            fontSize: monoSize,
            lineHeight: 1.5,
            color: dark.fg,
            letterSpacing: 0.5,
            wordBreak: "break-all",
            minHeight: vertical ? monoSize * 1.5 * 3 : monoSize * 1.5,
            maxWidth: 1500,
          }}
        >
          {typed}
          <span style={{ opacity: cursorOn ? 0.8 : 0, color: dark.accent }}>▍</span>
        </div>
        <Reveal delay={104}>
          <div style={{ fontFamily: fonts.sans, fontSize: type.bodySmall, color: dark.muted, marginTop: 36, maxWidth: 1200, lineHeight: 1.35 }}>
            an ordinary shielded transaction: no amount, no sender, no memo
          </div>
        </Reveal>
      </AbsoluteFill>
    </Backdrop>
  );
};
