import React from "react";
import { interpolate, useCurrentFrame } from "remotion";
import { dark, fonts, type } from "../theme";

export type CaptionItem = { text: string; at: number };

type Props = {
  items: CaptionItem[]; // each caption appears at frame `at` and stays until the next one
  height?: number;
};

// Caption bar at the bottom: a translucent dark band, 40px, at most two lines.
export const Caption: React.FC<Props> = ({ items, height = 160 }) => {
  const frame = useCurrentFrame();
  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        bottom: 0,
        height,
        backgroundColor: "rgba(27,21,17,0.82)",
        borderTop: `1px solid ${dark.hairline}`,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      {items.map((it, i) => {
        const next = items[i + 1]?.at ?? Number.POSITIVE_INFINITY;
        const fadeIn = interpolate(frame, [it.at, it.at + 14], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const fadeOut = interpolate(frame, [next - 10, next], [1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const y = interpolate(frame, [it.at, it.at + 14], [10, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
        const opacity = Math.min(fadeIn, fadeOut);
        if (opacity <= 0) return null;
        return (
          <div
            key={i}
            style={{
              position: "absolute",
              maxWidth: 1560,
              padding: "0 80px",
              textAlign: "center",
              fontFamily: fonts.sans,
              fontSize: type.body,
              lineHeight: 1.3,
              color: dark.fg,
              opacity,
              transform: `translateY(${y}px)`,
            }}
          >
            {it.text}
          </div>
        );
      })}
    </div>
  );
};
