import { describe, expect, it } from "vitest";
import { appName } from "./branding";

describe("branding", () => {
  it("exposes the app name", () => {
    expect(appName).toBe("Spotify Archivist");
  });
});
