import React from "react";
import { AbsoluteFill, useCurrentFrame, useVideoConfig } from "remotion";
import { dark, inks } from "../theme";

type Props = {
  seed?: number;
  children?: React.ReactNode;
  tint?: number; // 0..1 how much ink shows
};

// Warm dark paper with a few soft ellipses, like the site's lot plates, drifting very slowly.
export const Backdrop: React.FC<Props> = ({ seed = 0, children, tint = 1 }) => {
  const frame = useCurrentFrame();
  const { width, height } = useVideoConfig();
  const t = frame / 30;
  const shapes = [
    { x: 0.18, y: 0.3, rx: 0.34, ry: 0.22, ink: inks[(seed + 0) % inks.length], op: 0.1, ph: 0 },
    { x: 0.78, y: 0.72, rx: 0.3, ry: 0.26, ink: inks[(seed + 2) % inks.length], op: 0.08, ph: 2.1 },
    { x: 0.6, y: 0.18, rx: 0.22, ry: 0.16, ink: inks[(seed + 4) % inks.length], op: 0.06, ph: 4.2 },
  ];
  return (
    <AbsoluteFill style={{ backgroundColor: dark.bg, overflow: "hidden" }}>
      {shapes.map((s, i) => {
        const dx = Math.sin(t * 0.09 + s.ph) * 0.012 * width;
        const dy = Math.cos(t * 0.07 + s.ph) * 0.012 * height;
        const w = s.rx * 2 * width;
        const h = s.ry * 2 * height;
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              left: s.x * width - w / 2 + dx,
              top: s.y * height - h / 2 + dy,
              width: w,
              height: h,
              borderRadius: "50%",
              background: `radial-gradient(ellipse at center, ${s.ink} 0%, transparent 68%)`,
              opacity: s.op * tint,
            }}
          />
        );
      })}
      <AbsoluteFill
        style={{
          background: "linear-gradient(180deg, rgba(27,21,17,0) 0%, rgba(27,21,17,0.35) 100%)",
        }}
      />
      {children}
    </AbsoluteFill>
  );
};
