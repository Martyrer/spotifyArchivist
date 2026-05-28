import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { Row } from "@/lib/ipc/types";
import { parseArtists } from "@/lib/ipc/types";
import { Ghost } from "lucide-react";
import { cn } from "@/lib/cn";

const ROW_HEIGHT = 56;

type Props = {
  rows: Row[];
  isLoading: boolean;
};

export function TrackList({ rows, isLoading }: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  if (isLoading) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-neutral-500">
        Loading…
      </div>
    );
  }
  if (rows.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-neutral-500">
        Nothing here yet. Run a sync.
      </div>
    );
  }

  const items = virtualizer.getVirtualItems();
  return (
    <div ref={parentRef} className="flex-1 overflow-y-auto">
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
                "flex items-center gap-3 border-b border-neutral-900 px-6",
                r.is_removed && "opacity-50 grayscale",
              )}
            >
              <div className="w-8 text-xs text-neutral-500">{vi.index + 1}</div>
              <div className="size-10 rounded bg-neutral-800" aria-hidden="true" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm">{r.name}</div>
                <div className="truncate text-xs text-neutral-500">
                  {artists.map((a) => a.name).join(", ")}
                </div>
              </div>
              <div className="hidden w-1/3 truncate text-xs text-neutral-500 md:block">
                {r.album}
              </div>
              {r.is_removed ? (
                <span
                  aria-label="removed by Spotify"
                  title="removed by Spotify"
                  className="text-rose-400"
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
