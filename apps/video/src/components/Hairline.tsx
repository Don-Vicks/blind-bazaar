import React from "react";
import { interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { dark, SPRING_SLOW } from "../theme";

type Props = {
  delay?: number;
  width?: number | string;
  color?: string;
  align?: "left" | "center";
  style?: React.CSSProperties;
};

// The one accent: a 1px line that draws across.
export const Hairline: React.FC<Props> = ({ delay = 0, width = "100%", color = dark.hairlineStrong, align = "left", style }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const p = spring({ frame: frame - delay, fps, config: SPRING_SLOW, durationInFrames: 60 });
  const sx = interpolate(p, [0, 1], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <div
      style={{
        width,
        height: 1,
        backgroundColor: color,
        transform: `scaleX(${sx})`,
        transformOrigin: align === "left" ? "left center" : "center",
        ...style,
      }}
    />
  );
};
