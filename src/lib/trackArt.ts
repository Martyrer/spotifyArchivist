// Deterministic abstract cover art generated from track metadata.
// Same seed always yields the same composition — no network, no storage.

export type ArtShape =
  | { kind: "rect"; x: number; y: number; w: number; h: number; color: string }
  | { kind: "circle"; cx: number; cy: number; r: number; color: string }
  | { kind: "tri"; points: string; color: string };

export interface ArtSpec {
  bg: string;
  shapes: ArtShape[];
}

const SIZE = 40;
const CELL = 8; // 5x5 grid
const KINDS = ["rect", "circle", "tri"] as const;

// FNV-1a 32-bit — cheap, well-distributed string hash.
function hashString(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

// mulberry32 — tiny deterministic PRNG seeded from the hash.
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const snap = (rng: () => number, max: number) => Math.round(rng() * max) * CELL;

export function trackArtSeed(name: string, artists: string, album: string): string {
  return `${name}|${artists}|${album}`;
}

export function generateTrackArt(seed: string): ArtSpec {
  const rng = mulberry32(hashString(seed));
  const hue = Math.floor(rng() * 360);
  const shift = [40, 150, 180, 210][Math.floor(rng() * 4)];

  const bg = `hsl(${hue} 32% 22%)`;
  const palette = [
    `hsl(${hue} 55% 58%)`,
    `hsl(${(hue + shift) % 360} 50% 60%)`,
    `hsl(${(hue + shift) % 360} 60% 78%)`,
  ];

  const count = 3 + Math.floor(rng() * 3); // 3..5 shapes
  const shapes: ArtShape[] = [];
  for (let i = 0; i < count; i++) {
    const kind = KINDS[Math.floor(rng() * KINDS.length)];
    const color = palette[Math.floor(rng() * palette.length)];
    if (kind === "circle") {
      shapes.push({
        kind,
        cx: snap(rng, 4),
        cy: snap(rng, 4),
        r: CELL + Math.floor(rng() * 2) * CELL,
        color,
      });
    } else if (kind === "rect") {
      shapes.push({
        kind,
        x: snap(rng, 3),
        y: snap(rng, 3),
        w: CELL + Math.floor(rng() * 3) * CELL,
        h: CELL + Math.floor(rng() * 3) * CELL,
        color,
      });
    } else {
      const x = snap(rng, 3);
      const y = snap(rng, 3);
      const s = CELL * (1 + Math.floor(rng() * 2));
      shapes.push({ kind, points: `${x},${y + s} ${x + s},${y + s} ${x + s / 2},${y}`, color });
    }
  }
  return { bg, shapes };
}

export const ART_SIZE = SIZE;
