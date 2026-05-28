import { describe, expect, it, vi } from "vitest";
import { __setInvokeForTests, ipc } from "./client";

type AnyFn = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

const setInvoke = (fn: AnyFn) => __setInvokeForTests(fn as never);

describe("ipc client", () => {
  it("parses list_sources via zod schema", async () => {
    setInvoke(async () => [
      { id: 1, kind: "liked_songs", spotify_id: "__self__", name: "Liked Songs", enabled: true },
    ]);
    const sources = await ipc.list_sources();
    expect(sources[0].kind).toBe("liked_songs");
  });

  it("rejects malformed responses", async () => {
    setInvoke(async () => [{ id: "not a number" }]);
    await expect(ipc.list_sources()).rejects.toThrow(/invalid response/);
  });

  it("forwards args through to invoke", async () => {
    const spy = vi.fn(async () => undefined);
    setInvoke(spy);
    await ipc.toggle_source(7, false);
    expect(spy).toHaveBeenCalledWith("toggle_source", { id: 7, enabled: false });
  });

  it("parses settings shape", async () => {
    setInvoke(async () => ({
      sync_interval_hours: 6,
      authenticated: true,
      user_id: "u1",
    }));
    const s = await ipc.get_settings();
    expect(s.sync_interval_hours).toBe(6);
    expect(s.user_id).toBe("u1");
  });
});
