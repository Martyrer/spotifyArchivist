import { describe, expect, it } from "vitest";
import { ART_SIZE, generateTrackArt, trackArtSeed } from "./trackArt";

describe("trackArt", () => {
  it("is deterministic for the same seed", () => {
    const seed = trackArtSeed("Monsters", '[{"name":"James Blunt"}]', "Monsters");
    expect(generateTrackArt(seed)).toEqual(generateTrackArt(seed));
  });

  it("differs across distinct seeds", () => {
    const a = generateTrackArt(trackArtSeed("A", "x", "y"));
    const b = generateTrackArt(trackArtSeed("B", "x", "y"));
    expect(a).not.toEqual(b);
  });

  it("produces 3..5 shapes with in-bounds coords and a bg", () => {
    const spec = generateTrackArt(trackArtSeed("Doomed", "Maphra", "Doomed"));
    expect(spec.bg).toMatch(/^hsl\(/);
    expect(spec.shapes.length).toBeGreaterThanOrEqual(3);
    expect(spec.shapes.length).toBeLessThanOrEqual(5);
    for (const s of spec.shapes) {
      if (s.kind === "rect") {
        expect(s.x).toBeGreaterThanOrEqual(0);
        expect(s.y).toBeGreaterThanOrEqual(0);
        expect(s.w).toBeGreaterThan(0);
      } else if (s.kind === "circle") {
        expect(s.r).toBeGreaterThan(0);
      } else {
        expect(s.points.split(" ")).toHaveLength(3);
      }
    }
  });

  it("covers all three shape kinds across many seeds", () => {
    const kinds = new Set<string>();
    for (let i = 0; i < 200; i++) {
      for (const s of generateTrackArt(trackArtSeed(`t${i}`, "a", "b")).shapes) {
        kinds.add(s.kind);
      }
    }
    expect(kinds).toEqual(new Set(["rect", "circle", "tri"]));
  });

  it("exposes a fixed canvas size", () => {
    expect(ART_SIZE).toBe(40);
  });
});
