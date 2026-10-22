import React from "react";
import { Easing, IFrame, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { dark, fonts, light, SPRING, site, type, useIframe } from "../theme";

export type PageFallback = { title: string; lines: string[] };

type Props = {
  path: string; // "/a/jck1vba7"
  label: string; // text in the top strip
  width: number;
  height: number;
  pageScale?: number; // the page is laid out at width/pageScale and scaled up for readability
  delay?: number;
  drift?: boolean; // 1-2% scale drift across the scene so the page never sits still
  pan?: number; // page pixels to pan down over the scene (slow slide; sticky headers scroll away)
  fallback: PageFallback;
  style?: React.CSSProperties;
};

const BAR = 44;

// A thin paper sheet with a 24px corner: the live page from the running app, or a composed card.
export const DeviceFrame: React.FC<Props> = ({
  path,
  label,
  width,
  height,
  pageScale = 1.25,
  delay = 0,
  drift = true,
  pan = 0,
  fallback,
  style,
}) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const p = spring({ frame: frame - delay, fps, config: SPRING, durationInFrames: 45 });
  const opacity = interpolate(p, [0, 1], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const y = interpolate(p, [0, 1], [40, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const scale = drift
    ? interpolate(frame, [0, durationInFrames], [1, 1.015], {
        easing: Easing.inOut(Easing.sin),
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : 1;
  const innerW = Math.round(width / pageScale);
  // The page viewport is taller than the visible area so a fixed badge at its bottom (the Next dev
  // indicator) is clipped away.
  const innerH = Math.round((height - BAR) / pageScale) + pan + 72;
  const scrollY = interpolate(frame, [20, durationInFrames - 20], [0, pan], {
    easing: Easing.inOut(Easing.sin),
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <div
      style={{
        width,
        height,
        borderRadius: 24,
        overflow: "hidden",
        border: `1px solid ${dark.hairlineStrong}`,
        backgroundColor: light.bg,
        boxShadow: "0 40px 90px rgba(0,0,0,0.45)",
        opacity,
        transform: `translateY(${y}px) scale(${scale})`,
        transformOrigin: "center center",
        position: "relative",
        ...style,
      }}
    >
      <div
        style={{
          height: BAR,
          display: "flex",
          alignItems: "center",
          padding: "0 28px",
          gap: 14,
          borderBottom: `1px solid ${light.hairline}`,
          backgroundColor: light.surface,
          fontFamily: fonts.mono,
          fontSize: type.label,
          color: light.muted,
        }}
      >
        <span style={{ width: 10, height: 10, borderRadius: 5, backgroundColor: light.hairline }} />
        <span style={{ width: 10, height: 10, borderRadius: 5, backgroundColor: light.hairline }} />
        <span style={{ marginLeft: 10 }}>{label}</span>
      </div>
      <div style={{ position: "absolute", top: BAR, left: 0, width, height: height - BAR, overflow: "hidden" }}>
        {useIframe ? (
          <IFrame
            src={`${site}${path}`}
            scrolling="no"
            style={{
              width: innerW,
              height: innerH,
              border: 0,
              transform: `translateY(${-scrollY * pageScale}px) scale(${pageScale})`,
              transformOrigin: "top left",
              backgroundColor: light.bg,
            }}
          />
        ) : (
          <FallbackCard fallback={fallback} />
        )}
      </div>
    </div>
  );
};

const FallbackCard: React.FC<{ fallback: PageFallback }> = ({ fallback }) => (
  <div style={{ padding: "56px 72px", color: light.fg }}>
    <div style={{ fontFamily: fonts.serif, fontSize: type.h2, lineHeight: 1.1, marginBottom: 32 }}>{fallback.title}</div>
    {fallback.lines.map((l, i) => (
      <div
        key={i}
        style={{
          fontFamily: fonts.sans,
          fontSize: type.bodySmall,
          lineHeight: 1.4,
          color: light.muted,
          paddingTop: 18,
          paddingBottom: 18,
          borderTop: `1px solid ${light.hairline}`,
        }}
      >
        {l}
      </div>
    ))}
  </div>
);
