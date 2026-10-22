import React from "react";
import { Composition } from "remotion";
import { TransitionSeries, linearTiming } from "@remotion/transitions";
import { fade } from "@remotion/transitions/fade";
import { Whoosh } from "./components/Whoosh";
import { Hook } from "./scenes/Hook";
import { Statements } from "./scenes/Statements";
import { LotPage } from "./scenes/LotPage";
import { ChainCloses } from "./scenes/ChainCloses";
import { ProofCards } from "./scenes/ProofCards";
import { GoDark } from "./scenes/GoDark";
import { EndCard } from "./scenes/EndCard";
import { TitleCard } from "./scenes/TitleCard";
import { PageScene } from "./scenes/PageScene";
import { TerminalCard } from "./scenes/TerminalCard";
import { SettlementCard } from "./scenes/SettlementCard";
import { HonestyTable } from "./scenes/HonestyTable";
import { LOT } from "./theme";

const FPS = 30;
const FADE = 18;
const PREMOUNT = 100; // pages are client-rendered; give them time to fetch before they are seen

type Scene = { d: number; el: React.ReactNode; premount?: number };

const total = (scenes: Scene[]) => scenes.reduce((n, s) => n + s.d, 0) - FADE * (scenes.length - 1);

// Scenes dissolve into each other; a soft whoosh marks each change.
const Series: React.FC<{ scenes: Scene[] }> = ({ scenes }) => (
  <TransitionSeries>
    {scenes.flatMap((s, i) => {
      const seq = (
        <TransitionSeries.Sequence key={`s${i}`} durationInFrames={s.d} premountFor={s.premount ?? 0}>
          {i > 0 ? <Whoosh /> : null}
          {s.el}
        </TransitionSeries.Sequence>
      );
      if (i === 0) return [seq];
      return [
        <TransitionSeries.Transition key={`t${i}`} presentation={fade()} timing={linearTiming({ durationInFrames: FADE })} />,
        seq,
      ];
    })}
  </TransitionSeries>
);

// Promo, 60 s: hook, proof, call to action.
const PROMO: Scene[] = [
  { d: 138, el: <Hook /> },
  { d: 258, el: <Statements /> },
  { d: 378, el: <LotPage />, premount: PREMOUNT },
  { d: 318, el: <ChainCloses /> },
  { d: 318, el: <ProofCards /> },
  { d: 258, el: <GoDark />, premount: PREMOUNT },
  { d: 240, el: <EndCard /> },
];

// Vertical cut, 30 s: scenes 1, 2, 4, 7.
const VERTICAL: Scene[] = [
  { d: 168, el: <Hook /> },
  { d: 258, el: <Statements /> },
  { d: 300, el: <ChainCloses /> },
  { d: 228, el: <EndCard /> },
];

// Demo, 150 s: the take table from docs/DEMO_SCRIPT.md with live pages and captions.
const DEMO: Scene[] = [
  { d: 240, el: <TitleCard /> },
  {
    d: 390,
    premount: PREMOUNT,
    el: (
      <PageScene
        path="/"
        pan={420}
        label="blind bazaar  ·  Zcash testnet"
        seed={2}
        captions={[
          { at: 10, text: "A sealed-bid auction house on Zcash testnet. No contract: a lot is a fresh shielded address and a viewing key." },
          { at: 200, text: "Every bid is a shielded payment with the bid inside its encrypted memo. The chain is the mailbox; the viewing key is the inbox." },
        ]}
        fallback={{ title: "Blind Bazaar", lines: ["Sealed-bid auctions on Zcash.", "Every bid is a shielded payment.", "Losers are never revealed."] }}
      />
    ),
  },
  {
    d: 450,
    premount: PREMOUNT,
    el: (
      <PageScene
        path={`/a/${LOT}`}
        pan={520}
        label={`/a/${LOT}  ·  Zcash testnet`}
        seed={3}
        captions={[
          { at: 10, text: "No contract. The lot is a fresh shielded address. Each bid is one private payment with the bid inside its encrypted memo." },
          { at: 230, text: "These two bids came from a wallet that is not ours." },
        ]}
        fallback={{ title: `Lot ${LOT}`, lines: ["First price. Reserve 0.02 TAZ, deposit 0.01 TAZ.", "Two receipts from an independent wallet.", "Closed by the chain."] }}
      />
    ),
  },
  {
    d: 390,
    premount: PREMOUNT,
    el: (
      <PageScene
        path={`/a/${LOT}/status`}
        pan={480}
        label={`/a/${LOT}/status`}
        seed={4}
        captions={[
          { at: 10, text: "Receipts are sha256 of the bidder's nonce: a bidder sees \"bid received\" without revealing anything." },
          { at: 200, text: "A Merkle root over every valid bid is published before any refund; each bidder verifies their own leaf." },
        ]}
        fallback={{ title: "Status", lines: ["Two receipts.", "Ledger root published at close.", "Each bidder verifies their own leaf."] }}
      />
    ),
  },
  {
    d: 390,
    el: (
      <TerminalCard
        captions={[
          { at: 10, text: "On the chain it is an ordinary shielded transaction: no amount, no sender, no memo." },
          { at: 220, text: "The chain closes it, not our clock. A bid mined after the close block is late and refunded." },
        ]}
      />
    ),
  },
  {
    d: 450,
    premount: PREMOUNT,
    el: (
      <PageScene
        path={`/a/${LOT}/result`}
        pan={520}
        label={`/a/${LOT}/result`}
        seed={5}
        captions={[
          { at: 10, text: "A wins and pays 0.03 TAZ. The ledger root was committed at close; the losing bid stays sealed, even from the seller." },
          { at: 250, text: "The result publishes a note-opening disclosure of the winning payment and the close height." },
        ]}
        fallback={{ title: "Result", lines: ["Winner pays 0.03 TAZ.", "Ledger root committed at close.", "Winning payment disclosed; the losing bid stays sealed."] }}
      />
    ),
  },
  {
    d: 390,
    premount: PREMOUNT,
    el: (
      <PageScene
        path={`/a/${LOT}/verify`}
        pan={120}
        label={`/a/${LOT}/verify`}
        seed={0}
        captions={[
          { at: 10, text: "Anyone can rebuild the winning payment's commitment from the published opening and check it against the chain." },
          { at: 210, text: "Nothing about the loser is revealed." },
        ]}
        fallback={{ title: "Verify", lines: ["Recompute the note commitment in the browser.", "Compare it with the chain.", "Nothing about the loser is revealed."] }}
      />
    ),
  },
  {
    d: 330,
    el: (
      <SettlementCard
        captions={[
          { at: 10, text: "The winner paid the 0.02 balance by memo; the seller was paid 0.03; the loser's deposit came back after a random delay." },
          { at: 190, text: "On the chain these are three ordinary shielded transactions. All of it on Zcash testnet." },
        ]}
      />
    ),
  },
  {
    d: 450,
    premount: PREMOUNT,
    el: (
      <PageScene
        path="/embed"
        pan={80}
        label="/embed  ·  Go Dark"
        seed={1}
        captions={[
          { at: 10, text: "The same rails are a private pay button for any site. A payer on Solana gets a live quote into shielded ZEC." },
          { at: 200, text: "A recipient with no wallet makes one here; nothing leaves the page. Mainnet rails are not exercised in this take." },
        ]}
        fallback={{ title: "Go Dark", lines: ["A pay button for any site.", "Live quotes from Solana and Ethereum rails.", "Make a wallet in this browser."] }}
      />
    ),
  },
  {
    d: 330,
    premount: PREMOUNT,
    el: (
      <PageScene
        path="/how"
        pan={400}
        label="/how"
        seed={2}
        captions={[
          { at: 10, text: "Four formats on the same rails: first price, editions, Vickrey, request for quotes. Sealed-reveal lots keep bids from the operator until reveal." },
        ]}
        fallback={{ title: "How it works", lines: ["First price, editions, Vickrey, request for quotes.", "Sealed-reveal: commitments the operator cannot open early."] }}
      />
    ),
  },
  {
    d: 510,
    el: (
      <HonestyTable
        captions={[
          { at: 10, text: "What is private, what is real, what is not: on classic lots the operator reads bids while a lot is open; sealed-reveal lots remove that." },
          { at: 300, text: "We say so." },
        ]}
      />
    ),
  },
  { d: 378, el: <EndCard /> },
];

export const RemotionRoot: React.FC = () => (
  <>
    <Composition id="Promo" component={() => <Series scenes={PROMO} />} durationInFrames={total(PROMO)} fps={FPS} width={1920} height={1080} />
    <Composition id="Demo" component={() => <Series scenes={DEMO} />} durationInFrames={total(DEMO)} fps={FPS} width={1920} height={1080} />
    <Composition id="PromoVertical" component={() => <Series scenes={VERTICAL} />} durationInFrames={total(VERTICAL)} fps={FPS} width={1080} height={1920} />
  </>
);
