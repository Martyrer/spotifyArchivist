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
      last_sync_at: "2026-01-01T00:00:00Z",
      authenticated: true,
      user_id: "u1",
      onboarded: true,
    }));
    const s = await ipc.get_settings();
    expect(s.sync_interval_hours).toBe(6);
    expect(s.user_id).toBe("u1");
  });

  it("ping forwards through invoke", async () => {
    setInvoke(async () => "pong");
    expect(await ipc.ping()).toBe("pong");
  });

  it("update_settings sends camelCase arg key", async () => {
    const spy = vi.fn(async () => ({
      sync_interval_hours: 4,
      last_sync_at: null,
      authenticated: false,
      user_id: null,
      onboarded: false,
    }));
    setInvoke(spy);
    await ipc.update_settings(4);
    expect(spy).toHaveBeenCalledWith("update_settings", { syncIntervalHours: 4 });
  });

  it("list_memberships forwards filter and sourceId", async () => {
    const spy = vi.fn(async () => []);
    setInvoke(spy);
    await ipc.list_memberships(7, "removed");
    expect(spy).toHaveBeenCalledWith("list_memberships", {
      sourceId: 7,
      filter: "removed",
    });
  });

  it("logout / mark_seen / cancel_login / get_unseen_losses round-trip", async () => {
    const spy = vi.fn(async () => undefined);
    setInvoke(spy);
    await ipc.logout();
    await ipc.reset_app();
    await ipc.mark_seen();
    await ipc.cancel_login();
    expect(spy).toHaveBeenCalledWith("reset_app");
    setInvoke(async () => 7);
    expect(await ipc.get_unseen_losses()).toBe(7);
  });

  it("track_playlist passes spotifyId and name", async () => {
    const spy = vi.fn(async () => 99);
    setInvoke(spy);
    const id = await ipc.track_playlist("pl1", "Mix");
    expect(id).toBe(99);
    expect(spy).toHaveBeenCalledWith("track_playlist", {
      spotifyId: "pl1",
      name: "Mix",
    });
  });

  it("trigger_sync parses outcomes", async () => {
    setInvoke(async () => [
      { source_id: 1, newly_lost: ["t1"], newly_pending: [], cleared_pending: [], total_present: 5 },
    ]);
    const out = await ipc.trigger_sync();
    expect(out[0].newly_lost).toEqual(["t1"]);
  });

  it("export forwards scope and path", async () => {
    const spy = vi.fn(async () => 42);
    setInvoke(spy);
    const written = await ipc.export({ kind: "all" }, "/tmp/out.jsonl");
    expect(written).toBe(42);
    expect(spy).toHaveBeenCalledWith("export", {
      scope: { kind: "all" },
      path: "/tmp/out.jsonl",
    });
  });

  it("start_login + await_login parse responses", async () => {
    setInvoke(async () => ({ authorize_url: "https://accounts.spotify.com/x" }));
    expect((await ipc.start_login()).authorize_url).toContain("accounts.spotify.com");
    setInvoke(async () => ({
      sync_interval_hours: 6,
      last_sync_at: null,
      authenticated: true,
      user_id: "u1",
      onboarded: true,
    }));
    expect((await ipc.await_login()).authenticated).toBe(true);
  });

  it("list_available_playlists parses array", async () => {
    setInvoke(async () => [
      { id: "p1", name: "Mix", already_tracked: false },
    ]);
    const out = await ipc.list_available_playlists();
    expect(out[0].id).toBe("p1");
  });
});
