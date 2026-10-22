import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { DeviceFrame } from "../components/DeviceFrame";
import { Hairline } from "../components/Hairline";
import { Reveal } from "../components/Reveal";
import { dark, fonts, type } from "../theme";

// Go Dark: the headline on the left, the live /embed page on the right.
export const GoDark: React.FC = () => (
  <Backdrop seed={5}>
    <AbsoluteFill style={{ flexDirection: "row", alignItems: "center", padding: "0 120px", gap: 72 }}>
      <div style={{ width: 640, flexShrink: 0 }}>
        <Reveal delay={0}>
          <div style={{ fontFamily: fonts.mono, fontSize: type.label, color: dark.accent, marginBottom: 24 }}>Go Dark</div>
        </Reveal>
        <Reveal delay={8}>
          <div style={{ fontFamily: fonts.serif, fontSize: type.h2, lineHeight: 1.12, color: dark.fg }}>
            Pay or bid from Solana or Ethereum. <span style={{ fontStyle: "italic", color: dark.accent }}>Land in shielded ZEC.</span>
          </div>
        </Reveal>
        <Hairline delay={24} width={200} style={{ margin: "40px 0" }} />
        <Reveal delay={40}>
          <div style={{ fontFamily: fonts.sans, fontSize: type.caption, lineHeight: 1.4, color: dark.muted }}>
            Live quotes. A wallet made in the browser. Mainnet swaps not exercised in this build.
          </div>
        </Reveal>
      </div>
      <DeviceFrame
        path="/embed"
        label="/embed  ·  Go Dark"
        width={1000}
        height={760}
        pageScale={1.15}
        delay={10}
        fallback={{
          title: "Go Dark",
          lines: ["A pay button for any site.", "Solana through NEAR Intents; Ethereum through Maya.", "Make a wallet in this browser: 24 words, nothing stored or sent."],
        }}
      />
    </AbsoluteFill>
  </Backdrop>
);
