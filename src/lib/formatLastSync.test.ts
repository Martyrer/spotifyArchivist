import { describe, expect, it } from "vitest";
import { formatLastSync } from "./formatLastSync";

describe("formatLastSync", () => {
  it("shows Never when no sync has completed", () => {
    expect(formatLastSync(null)).toBe("Never");
  });

  it("formats a valid timestamp with the current locale formatter", () => {
    const value = "2026-06-02T19:28:00Z";
    expect(formatLastSync(value)).toBe(
      new Intl.DateTimeFormat(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(new Date(value)),
    );
  });

  it("returns the raw value when the timestamp is invalid", () => {
    expect(formatLastSync("not-a-date")).toBe("not-a-date");
  });
});
