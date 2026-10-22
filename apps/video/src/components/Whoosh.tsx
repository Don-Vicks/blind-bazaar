import React from "react";
import { Audio, Sequence } from "remotion";
import { whoosh } from "@remotion/sfx";

// A soft whoosh at the start of a scene. Used on every scene change; no music.
export const Whoosh: React.FC<{ volume?: number }> = ({ volume = 0.16 }) => (
  <Sequence from={0} durationInFrames={40}>
    <Audio src={whoosh} volume={volume} />
  </Sequence>
);
