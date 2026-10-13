# Brand — Blind Bazaar

_Status: active (Oct 13, 2026)_

Direction: luxury, refined, dark-native. A private auction house, not a dashboard.

## Palette (warm monochrome + one accent)

| Token | Dark | Light | Role |
|---|---|---|---|
| background | #121110 | #F6F3EE | page |
| surface | #181715 | #FFFDFA | raised panels (bid panel, result) |
| foreground | #EDE8DF | #1A1816 | text |
| muted-foreground | #9A9288 | #6A635A | secondary text |
| hairline | rgba(237,232,223,0.10) | rgba(26,24,22,0.12) | dividers, input borders |
| accent | #C9A961 (brass) | #8B6D2B | live state, links, focus, countdown |
| danger | #E07A6B | #B4382A | errors |
| success | #8FBF9A | #2F7A4A | received / sent |

Accent appears on at most one element per view. Never gradients. Never pure black.

## Typography

- Display and lot titles: Instrument Serif (next/font), tracking -0.01em, weights 400 only.
- UI: Geist Sans, 3 weights (400, 500, 600).
- Numbers, addresses, receipts, memos: Geist Mono, `tabular-nums`.
- Micro-labels: 11px uppercase, tracking 0.14em, muted.
- Scale: 11 / 13 / 15 / 18 / 28 / 44 / 72px.

## Layout

- Max width 1120px, 24px gutters, 8px grid. Sections separated by hairlines, 48 to 96px apart.
- Lot page is asymmetric: 7/12 narrative, 5/12 sticky facts and bid column.
- Radius: 2px on inputs and buttons, 6px on panels, full on dots only.
- Shadows: none. Depth comes from surface shade and hairlines.

## Motion

- 180ms ease-out for state changes, 300ms fade-up on first mount, nothing on exit beyond 120ms.
- Respect `prefers-reduced-motion`.

## Voice

Specific, calm, no hype. Say what the chain does. "Losing bids are never revealed" beats "military-grade privacy".
