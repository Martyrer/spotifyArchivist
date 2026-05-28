import type { MembershipFilter, Row, Source } from "@/lib/ipc/types";
import { cn } from "@/lib/cn";
import { Link } from "@tanstack/react-router";
import { Heart, ListMusic, RefreshCcw, Settings as SettingsIcon } from "lucide-react";
import { ipc } from "@/lib/ipc/client";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { TrackList } from "./TrackList";
import { FilterPill } from "./FilterPill";

type Props = {
  sources: Source[];
  activeId: number;
  sourceName: string;
  rows: Row[];
  isLoading: boolean;
  filter: MembershipFilter;
  onFilter: (f: MembershipFilter) => void;
};

export function SourceShell({
  sources,
  activeId,
  sourceName,
  rows,
  isLoading,
  filter,
  onFilter,
}: Props) {
  const qc = useQueryClient();
  const sync = useMutation({
    mutationFn: ipc.trigger_sync,
    onSettled: () => qc.invalidateQueries({ queryKey: ["memberships", activeId] }),
  });

  return (
    <div className="grid h-full w-full grid-cols-[260px_1fr]">
      <aside className="flex h-full flex-col border-r border-neutral-800 bg-neutral-900/40">
        <div className="px-4 py-4 text-sm font-semibold tracking-tight">Spotify Archivist</div>
        <nav className="flex-1 overflow-y-auto px-2 pb-4">
          {sources.map((s) => {
            const Icon = s.kind === "liked_songs" ? Heart : ListMusic;
            return (
              <Link
                key={s.id}
                to="/source/$id"
                params={{ id: String(s.id) }}
                className={cn(
                  "flex items-center gap-2 rounded-md px-2 py-1.5 text-sm",
                  s.id === activeId
                    ? "bg-neutral-800 text-neutral-50"
                    : "text-neutral-400 hover:bg-neutral-800/50",
                )}
              >
                <Icon size={14} />
                <span className="truncate">{s.name}</span>
              </Link>
            );
          })}
        </nav>
        <div className="border-t border-neutral-800 px-2 py-2">
          <Link
            to="/settings"
            className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-neutral-400 hover:bg-neutral-800/50"
          >
            <SettingsIcon size={14} />
            <span>Settings</span>
          </Link>
        </div>
      </aside>

      <main className="flex h-full flex-col">
        <header className="flex items-center justify-between border-b border-neutral-800 px-6 py-3">
          <div>
            <h1 className="text-lg font-semibold tracking-tight">{sourceName}</h1>
            <p className="text-xs text-neutral-500">{rows.length} rows</p>
          </div>
          <div className="flex items-center gap-3">
            <FilterPill value={filter} onChange={onFilter} />
            <button
              type="button"
              onClick={() => sync.mutate()}
              disabled={sync.isPending}
              className="flex items-center gap-1 rounded-full border border-neutral-700 px-3 py-1 text-xs hover:bg-neutral-800 disabled:opacity-50"
            >
              <RefreshCcw size={12} />
              {sync.isPending ? "Syncing…" : "Sync now"}
            </button>
          </div>
        </header>
        <TrackList rows={rows} isLoading={isLoading} />
      </main>
    </div>
  );
}
