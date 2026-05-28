import { describe, expect, it } from "vitest";
import { parseArtists } from "./types";

describe("parseArtists", () => {
  it("parses a well-formed JSON array", () => {
    const a = parseArtists('[{"id":"a","name":"A"},{"id":null,"name":"B"}]');
    expect(a).toHaveLength(2);
    expect(a[0].name).toBe("A");
    expect(a[1].id).toBeNull();
  });

  it("returns [] for invalid JSON", () => {
    expect(parseArtists("not json")).toEqual([]);
  });

  it("returns [] when shape mismatches", () => {
    expect(parseArtists("[{\"id\": 7}]")).toEqual([]);
  });

  it("returns [] for non-array JSON", () => {
    expect(parseArtists("{}")).toEqual([]);
  });
});
