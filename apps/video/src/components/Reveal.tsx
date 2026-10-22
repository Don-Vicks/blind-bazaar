import React from "react";
import { interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { SPRING } from "../theme";

type Props = {
  delay?: number;
  distance?: number;
  style?: React.CSSProperties;
  children: React.ReactNode;
};

// The one entrance used everywhere: fade up, spring damping 200, no bounce.
export const Reveal: React.FC<Props> = ({ delay = 0, distance = 24, style, children }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const p = spring({ frame: frame - delay, fps, config: SPRING, durationInFrames: 40 });
  const opacity = interpolate(p, [0, 1], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const y = interpolate(p, [0, 1], [distance, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return <div style={{ opacity, transform: `translateY(${y}px)`, ...style }}>{children}</div>;
};
