import React from "react";
import { AbsoluteFill } from "remotion";
import { Backdrop } from "../components/Backdrop";
import { DeviceFrame } from "../components/DeviceFrame";
import { Reveal } from "../components/Reveal";
import { dark, fonts, LOT, type } from "../theme";

// The live lot page on a paper sheet, sliding up 40px while it fades in.
export const LotPage: React.FC = () => (
  <Backdrop seed={2}>
    <AbsoluteFill style={{ alignItems: "center", paddingTop: 84 }}>
      <DeviceFrame
        path={`/a/${LOT}`}
        label={`/a/${LOT}  ·  Zcash testnet`}
        width={1500}
        height={780}
        delay={0}
        pan={520}
        fallback={{
          title: `Lot ${LOT}`,
          lines: ["Tender. Budget 0.05 TAZ, bond 0.01 TAZ.", "Two receipts: quotes from an independent wallet.", "Closed by the chain."],
        }}
      />
      <Reveal delay={26} style={{ marginTop: 44 }}>
        <div style={{ fontFamily: fonts.sans, fontSize: type.bodySmall, color: dark.muted, textAlign: "center" }}>
          Tender {LOT} on Zcash testnet. Two sealed quotes from an independent supplier wallet.
        </div>
      </Reveal>
    </AbsoluteFill>
  </Backdrop>
);
