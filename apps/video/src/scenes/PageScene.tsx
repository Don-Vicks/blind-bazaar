import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { Caption, CaptionItem } from "../components/Caption";
import { DeviceFrame, PageFallback } from "../components/DeviceFrame";

type Props = {
  path: string;
  label: string;
  captions: CaptionItem[];
  fallback: PageFallback;
  seed?: number;
};

// Demo building block: one live page on a paper sheet with a caption band under it.
export const PageScene: React.FC<Props> = ({ path, label, captions, fallback, seed = 2 }) => (
  <Backdrop seed={seed} tint={0.7}>
    <AbsoluteFill style={{ alignItems: "center", paddingTop: 64 }}>
      <DeviceFrame path={path} label={label} width={1560} height={790} delay={0} fallback={fallback} />
    </AbsoluteFill>
    <Caption items={captions} />
  </Backdrop>
);
