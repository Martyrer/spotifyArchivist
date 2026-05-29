import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { Row } from "@/lib/ipc/types";
import { parseArtists } from "@/lib/ipc/types";
import { Ghost } from "lucide-react";
import { cn } from "@/lib/cn";
import { TrackArt } from "./TrackArt";
import { DotField } from "./DotField";

const ROW_HEIGHT = 56;

type Props = {
  rows: Row[];
  isLoading: boolean;
  isSyncing: boolean;
};

export function TrackList({ rows, isLoading, isSyncing }: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  if (isSyncing && rows.length === 0) {
    return (
      <div className="relative flex h-full items-center justify-center overflow-hidden bg-bg">
        <DotField cell={16} />
        <div className="relative z-10 flex flex-col items-center gap-2 text-sm text-muted">
          <span className="font-mono text-accent">Syncing…</span>
          <span className="text-xs text-faint">Fetching tracks from Spotify</span>
        </div>
      </div>
    );
  }
  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted">
        Loading…
      </div>
    );
  }
  if (rows.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted">
        Nothing here yet. Run a sync.
      </div>
    );
  }

  const items = virtualizer.getVirtualItems();
  return (
    <div ref={parentRef} className="h-full overflow-y-auto bg-bg">
      <div
        style={{ height: virtualizer.getTotalSize() }}
        className="relative w-full"
      >
        {items.map((vi) => {
          const r = rows[vi.index];
          const artists = parseArtists(r.artists);
          return (
            <div
              key={`${r.source_id}-${r.track_id}`}
              data-testid="track-row"
              data-removed={r.is_removed ? "true" : "false"}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                right: 0,
                transform: `translateY(${vi.start}px)`,
                height: vi.size,
              }}
              className={cn(
                "flex items-center gap-3 border-b border-border-2 px-4 transition-colors duration-200 ease-out hover:bg-surface-2",
                r.is_removed && "opacity-50 grayscale",
              )}
            >
              <div className="w-8 font-mono text-[11px] tabular-nums text-faint">
                {vi.index + 1}
              </div>
              <TrackArt
                name={r.name}
                artists={r.artists}
                album={r.album}
                className="size-10 shrink-0"
              />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm text-fg">{r.name}</div>
                <div className="truncate text-xs text-muted">
                  {artists.map((a) => a.name).join(", ")}
                </div>
              </div>
              <div className="hidden w-1/3 truncate text-xs text-muted md:block">
                {r.album}
              </div>
              {r.is_removed ? (
                <span
                  aria-label="removed by Spotify"
                  title="removed by Spotify"
                  className="text-muted"
                >
                  <Ghost size={16} />
                </span>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}
