import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  AvailablePlaylist,
  type ExportScope,
  type MembershipFilter,
  Row,
  Settings,
  Source,
  StartLoginResponse,
  SyncOutcome,
} from "./types";
import { z } from "zod";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

let invoke: Invoke = tauriInvoke as Invoke;

export function __setInvokeForTests(fn: Invoke) {
  invoke = fn;
}

async function call<T>(cmd: string, schema: z.ZodType<T>, args?: Record<string, unknown>): Promise<T> {
  const raw = await invoke<unknown>(cmd, args);
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    throw new Error(`IPC ${cmd}: invalid response (${parsed.error.message})`);
  }
  return parsed.data;
}

export const ipc = {
  ping: () => invoke<string>("ping"),
  list_sources: () => call("list_sources", z.array(Source)),
  toggle_source: (id: number, enabled: boolean) =>
    invoke<void>("toggle_source", { id, enabled }),
  untrack_source: (id: number) => invoke<void>("untrack_source", { id }),
  list_memberships: (sourceId: number, filter: MembershipFilter) =>
    call("list_memberships", z.array(Row), { sourceId, filter }),
  get_settings: () => call("get_settings", Settings),
  update_settings: (syncIntervalHours: number) =>
    call("update_settings", Settings, { syncIntervalHours }),
  logout: () => invoke<void>("logout"),
  reset_app: () => invoke<void>("reset_app"),
  list_available_playlists: () =>
    call("list_available_playlists", z.array(AvailablePlaylist)),
  track_playlist: (spotifyId: string, name: string) =>
    invoke<number>("track_playlist", { spotifyId, name }),
  trigger_sync: () => call("trigger_sync", z.array(SyncOutcome)),
  export: (scope: ExportScope, path: string) =>
    invoke<number>("export", { scope, path }),
  start_login: () => call("start_login", StartLoginResponse),
  cancel_login: () => invoke<void>("cancel_login"),
  await_login: () => call("await_login", Settings),
  mark_seen: () => invoke<void>("mark_seen"),
  get_unseen_losses: () => invoke<number>("get_unseen_losses"),
  complete_onboarding: () => invoke<void>("complete_onboarding"),
};
