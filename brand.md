# Brand — Blind Bazaar

_Status: active (Oct 13, 2026)_

Direction: warm editorial. A private auction house that reads like a well-set catalogue: paper tones, umber ink, one terracotta accent, imagery first. Dark and light both warm.

## Palette (warm monochrome + one accent)

| Token | Dark | Light | Role |
|---|---|---|---|
| background | #1B1511 (umber black) | #F3EBDF (paper) | page |
| surface | #231B16 | #FAF4EA | raised panels (bid panel, result) |
| foreground | #F1E7D8 | #231B14 | text |
| muted-foreground | #A69583 | #6E6153 | secondary text |
| hairline | rgba(241,231,216,0.11) | rgba(35,27,20,0.12) | dividers, input borders |
| accent | #D99A5B (terracotta) | #A8542C | live state, links, focus, drop caps |
| danger | #E07A6B | #B4382A | errors |
| success | #8FBF9A | #2F7A4A | received / sent |

Accent appears on at most one element per view. Never gradients. Never pure black.

## Typography

- Display and lot titles: Instrument Serif (next/font), tracking -0.01em, weights 400 only.
- UI: Geist Sans, 3 weights (400, 500, 600).
- Numbers, addresses, receipts, memos: Geist Mono, `tabular-nums`.
- Micro-labels: 11px uppercase, tracking 0.14em, muted.
- Scale: 11 / 13 / 15 / 18 / 28 / 44 / 72px.

## Imagery

- Every lot has a cover: the seller's https image, or a procedural paper-toned plate seeded by the lot id
  (`LotPlate`). Plates use a fixed ink palette (terracotta, ochre, olive, slate, umber) with multiply blending
  and a grain overlay. No stock photography, no AI art.
- Home is a gallery: two-column at desktop, image above title, facts beneath. The lot page opens on a 16/7
  full-width cover.

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
