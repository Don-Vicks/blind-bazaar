/**
 * Procedural "plate" for lots without an image: a warm, paper-toned still-life abstraction seeded by the
 * lot id, so every lot has a distinct, stable cover and the gallery never shows an empty box.
 */
function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}

function rng(seed: number) {
  let t = seed || 1;
  return () => {
    t ^= t << 13;
    t ^= t >>> 17;
    t ^= t << 5;
    return ((t >>> 0) % 10000) / 10000;
  };
}

const INKS = ["#b7542f", "#c99a4a", "#7f8b5a", "#5b5f8a", "#a0463a", "#d9b27a", "#6f5a3c"];

export function LotPlate({ id, className = "", ratio = "4/3" }: { id: string; className?: string; ratio?: string }) {
  const r = rng(hash(id));
  const paper = r() > 0.5 ? "#efe4d2" : "#e8dcc6";
  const shapes = Array.from({ length: 3 + Math.floor(r() * 3) }, () => ({
    cx: 18 + r() * 64,
    cy: 28 + r() * 50,
    rx: 10 + r() * 22,
    ry: 8 + r() * 20,
    rot: r() * 60 - 30,
    fill: INKS[Math.floor(r() * INKS.length)],
    op: 0.72 + r() * 0.22,
  }));
  const horizon = 58 + r() * 18;
  return (
    <svg
      viewBox="0 0 100 75"
      preserveAspectRatio="xMidYMid slice"
      className={className}
      style={{ aspectRatio: ratio }}
      role="img"
      aria-label="Lot cover"
    >
      <defs>
        <filter id={`grain-${id}`} x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" stitchTiles="stitch" result="n" />
          <feColorMatrix type="saturate" values="0" in="n" result="g" />
          <feComponentTransfer in="g" result="t">
            <feFuncA type="table" tableValues="0 0.08" />
          </feComponentTransfer>
          <feBlend in="SourceGraphic" in2="t" mode="multiply" />
        </filter>
      </defs>
      <rect width="100" height="75" fill={paper} />
      <rect y={horizon} width="100" height={75 - horizon} fill="#1b1511" opacity="0.08" />
      {shapes.map((s, i) => (
        <ellipse
          key={i}
          cx={s.cx}
          cy={s.cy}
          rx={s.rx}
          ry={s.ry}
          transform={`rotate(${s.rot} ${s.cx} ${s.cy})`}
          fill={s.fill}
          opacity={s.op}
          style={{ mixBlendMode: "multiply" }}
        />
      ))}
      <rect width="100" height="75" fill={paper} filter={`url(#grain-${id})`} opacity="0.5" style={{ mixBlendMode: "multiply" }} />
    </svg>
  );
}

export function LotImage({
  id,
  src,
  alt,
  className = "",
  ratio = "4/3",
}: {
  id: string;
  src?: string | null;
  alt: string;
  className?: string;
  ratio?: string;
}) {
  if (src) {
    return (
      // eslint-disable-next-line @next/next/no-img-element
      <img src={src} alt={alt} loading="lazy" className={`block h-auto w-full object-cover ${className}`} style={{ aspectRatio: ratio }} />
    );
  }
  return <LotPlate id={id} className={`block w-full ${className}`} ratio={ratio} />;
}
