import React from "react";
import { AbsoluteFill, interpolate, useCurrentFrame } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Caption, CaptionItem } from "../components/Caption";
import { Reveal } from "../components/Reveal";
import { dark, fonts, TX } from "../theme";

type Line = { text: string; kind: "cmd" | "out"; at: number; speed: number };

const LINES: Line[] = [
  { kind: "cmd", text: 'bb-auction wallet --dir ./bidder --seed-file ./bidder.seed --pay "zcash:…"', at: 20, speed: 1.6 },
  { kind: "out", text: `sent ${TX.bidA}`, at: 80, speed: 2.4 },
  { kind: "cmd", text: 'bb-auction wallet --dir ./bidder --seed-file ./bidder.seed --pay "zcash:…"', at: 130, speed: 1.6 },
  { kind: "out", text: `sent ${TX.bidB}`, at: 190, speed: 2.4 },
];

// The two bids from the bundled wallet, typed by string slicing. Memo never printed.
export const TerminalCard: React.FC<{ captions: CaptionItem[] }> = ({ captions }) => {
  const frame = useCurrentFrame();
  return (
    <Backdrop seed={3} tint={0.7}>
      <AbsoluteFill style={{ alignItems: "center", paddingTop: 64 }}>
        <Reveal delay={0} distance={40}>
          <div
            style={{
              width: 1560,
              height: 790,
              borderRadius: 24,
              border: `1px solid ${dark.hairlineStrong}`,
              backgroundColor: "#140f0c",
              boxShadow: "0 40px 90px rgba(0,0,0,0.45)",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                height: 44,
                display: "flex",
                alignItems: "center",
                padding: "0 28px",
                gap: 14,
                borderBottom: `1px solid ${dark.hairline}`,
                fontFamily: fonts.mono,
                fontSize: 28,
                color: dark.muted,
              }}
            >
              <span style={{ width: 10, height: 10, borderRadius: 5, backgroundColor: dark.hairlineStrong }} />
              <span style={{ width: 10, height: 10, borderRadius: 5, backgroundColor: dark.hairlineStrong }} />
              <span style={{ marginLeft: 10 }}>bidder — zsh</span>
            </div>
            <div style={{ padding: "44px 56px", fontFamily: fonts.mono, fontSize: 36, lineHeight: 1.65, color: dark.fg }}>
              {LINES.map((l, i) => {
                const n = Math.floor(
                  interpolate(frame, [l.at, l.at + l.text.length / l.speed], [0, l.text.length], {
                    extrapolateLeft: "clamp",
                    extrapolateRight: "clamp",
                  }),
                );
                if (frame < l.at) return null;
                const typing = n < l.text.length;
                return (
                  <div key={i} style={{ wordBreak: "break-all", color: l.kind === "out" ? dark.muted : dark.fg, marginBottom: l.kind === "out" ? 36 : 0 }}>
                    {l.kind === "cmd" && <span style={{ color: dark.accent }}>$ </span>}
                    {l.text.slice(0, n)}
                    {typing && frame % 24 < 14 && <span style={{ color: dark.accent }}>▍</span>}
                  </div>
                );
              })}
            </div>
          </div>
        </Reveal>
      </AbsoluteFill>
      <Caption items={captions} />
    </Backdrop>
  );
};
