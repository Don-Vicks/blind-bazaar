# @blind-bazaar/video

Remotion compositions for the ZECATHON submission. Three videos and two stills, all rendered from code
against the running web app, so the pages in the videos are the real pages.

| Composition | Size | Length | Output |
|---|---|---|---|
| `Promo` | 1920x1080, 30 fps | 60 s | `out/promo.mp4` |
| `Demo` | 1920x1080, 30 fps | 150 s | `out/demo.mp4` |
| `PromoVertical` | 1080x1920, 30 fps | 30 s | `out/promo-vertical.mp4` |
| stills | 1280x720 | | `out/promo-thumb.png`, `out/demo-thumb.png` |

## What must be running

The `Promo` and `Demo` compositions embed live pages with Remotion's `<IFrame>`:

- the web app at `http://localhost:3001` (`pnpm --filter web dev -p 3001`, or whatever port you set in
  `REMOTION_SITE_URL`), with
- the auction service behind it at `http://127.0.0.1:8787`, serving lot `jck1vba7` (closed and paid on the
  public testnet; see `docs/evidence/testnet-jck1vba7/`).

Pages used: `/`, `/a/jck1vba7`, `/a/jck1vba7/status`, `/a/jck1vba7/result`, `/a/jck1vba7/verify`, `/embed`,
`/how`. The pages are client-rendered, so every page scene is premounted 100 frames early and the render runs
one browser tab at a time (`Config.setConcurrency(1)`); do not raise the concurrency, a fresh tab mid-scene
would show a page still loading.

If the app is not running, set `REMOTION_NO_IFRAME=1` and every page frame becomes a composed card with the
same caption text.

The renderer's own HTTP server is pinned to port 4397 and the studio to 4398 so they never collide with the
app's ports.

## Render

From the repo root, after `pnpm install`:

```bash
pnpm --filter @blind-bazaar/video render:promo      # out/promo.mp4
pnpm --filter @blind-bazaar/video render:demo       # out/demo.mp4
pnpm --filter @blind-bazaar/video render:vertical   # out/promo-vertical.mp4
pnpm --filter @blind-bazaar/video stills            # out/promo-thumb.png, out/demo-thumb.png
pnpm --filter @blind-bazaar/video render            # all of the above
pnpm --filter @blind-bazaar/video studio            # preview at http://localhost:4398
```

Finals use `--codec h264 --crf 18`. For a quick check add `--crf 23 --scale 0.5` to any render command.
The first render downloads Remotion's headless Chrome. The whoosh on scene changes comes from
`@remotion/sfx` (fetched from remotion.media at render time); there is no music.

## Layout

```
src/index.ts            registerRoot
src/Root.tsx            the three compositions and their scene lists
src/theme.ts            colors from apps/web globals.css, fonts, spring, txids from the evidence folder
src/components/         Backdrop (dark paper, soft ellipses), Reveal (fade up), Hairline (draws across),
                        DeviceFrame (live page on a paper sheet, pan and drift), Caption (bottom band), Whoosh
src/scenes/             one file per scene
```

Every number and claim on screen comes from `README.md`, `docs/WRITEUP.md`, `docs/DEMO_SCRIPT.md` and
`docs/evidence/testnet-jck1vba7/txids.md`. If those change, change the scene.
