import { z } from "zod";

export const SourceKind = z.enum(["liked_songs", "playlist"]);
export type SourceKind = z.infer<typeof SourceKind>;

export const Source = z.object({
  id: z.number().int(),
  kind: SourceKind,
  spotify_id: z.string(),
  name: z.string(),
  enabled: z.boolean(),
});
export type Source = z.infer<typeof Source>;

export const Row = z.object({
  source_id: z.number().int(),
  track_id: z.string(),
  uri: z.string(),
  name: z.string(),
  artists: z.string(),
  album: z.string(),
  added_at: z.string(),
  position: z.number().int(),
  is_removed: z.boolean(),
  pending_vanish: z.boolean(),
});
export type Row = z.infer<typeof Row>;

export const MembershipFilter = z.enum(["all", "present", "removed"]);
export type MembershipFilter = z.infer<typeof MembershipFilter>;

export const Settings = z.object({
  sync_interval_hours: z.number().int(),
  last_sync_at: z.string().nullable(),
  authenticated: z.boolean(),
  user_id: z.string().nullable(),
  onboarded: z.boolean(),
});
export type Settings = z.infer<typeof Settings>;

export const AvailablePlaylist = z.object({
  id: z.string(),
  name: z.string(),
  already_tracked: z.boolean(),
});
export type AvailablePlaylist = z.infer<typeof AvailablePlaylist>;

export const SyncOutcome = z.object({
  source_id: z.number().int(),
  newly_lost: z.array(z.string()),
  newly_pending: z.array(z.string()),
  cleared_pending: z.array(z.string()),
  total_present: z.number().int(),
});
export type SyncOutcome = z.infer<typeof SyncOutcome>;

export const StartLoginResponse = z.object({
  authorize_url: z.string(),
});
export type StartLoginResponse = z.infer<typeof StartLoginResponse>;

export type ExportScope = { kind: "all" } | { kind: "source"; id: number };

export const TrackArtist = z.object({
  id: z.string().nullable().optional(),
  name: z.string(),
});
export type TrackArtist = z.infer<typeof TrackArtist>;

export function parseArtists(raw: string): TrackArtist[] {
  try {
    const arr = JSON.parse(raw);
    return z.array(TrackArtist).parse(arr);
  } catch {
    return [];
  }
}
