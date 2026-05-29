import { useEffect } from "react";
import type { MembershipFilter, Row, Source } from "@/lib/ipc/types";
import { cn } from "@/lib/cn";
import { Link, useNavigate } from "@tanstack/react-router";
import { Heart, ListMusic, RefreshCcw, Settings as SettingsIcon } from "lucide-react";
import { ipc } from "@/lib/ipc/client";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { TrackList } from "./TrackList";
import { FilterPill } from "./FilterPill";
import { ThemeToggle } from "./ThemeToggle";
import { PalettePopover } from "./PalettePopover";
import { DotField } from "./DotField";

type Props = {
  sources: Source[];
  activeId: number;
  sourceName: string;
  rows: Row[];
  isLoading: boolean;
  isSyncing: boolean;
  filter: MembershipFilter;
  onFilter: (f: MembershipFilter) => void;
};

export function SourceShell({
  sources,
  activeId,
  sourceName,
  rows,
  isLoading,
  isSyncing,
  filter,
  onFilter,
}: Props) {
  const qc = useQueryClient();
  const sync = useMutation({
    mutationFn: ipc.trigger_sync,
    onSettled: () => {
      qc.invalidateQueries({ queryKey: ["memberships"] });
      qc.invalidateQueries({ queryKey: ["sources"] });
    },
  });
  // Either this window kicked off the sync (mutation pending) or another
  // trigger (tray/scheduler) is running (global flag from the backend).
  const syncing = isSyncing || sync.isPending;
  const navigate = useNavigate();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      // Don't hijack arrows while typing or inside a native control.
      const t = e.target as HTMLElement | null;
      if (t && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName))) return;
      if (sources.length === 0) return;
      e.preventDefault();
      const idx = sources.findIndex((s) => s.id === activeId);
      const cur = idx === -1 ? 0 : idx;
      const step = e.key === "ArrowDown" ? 1 : -1;
      const next = (cur + step + sources.length) % sources.length;
      const target = sources[next];
      if (target.id !== activeId) {
        navigate({ to: "/source/$id", params: { id: String(target.id) } });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [sources, activeId, navigate]);

  return (
    <div className="grid h-screen w-screen grid-cols-[var(--sidebar-w)_1fr] overflow-hidden bg-bg text-fg">
      <aside className="relative grid h-screen grid-rows-[var(--row-h)_1fr_var(--row-h)] overflow-hidden border-r border-border bg-surface">
        <DotField cell={12} />
        <header className="hrow relative z-10 flex min-h-row items-center gap-2 border-b border-border px-3">
          <span className="grid size-6 place-items-center bg-surface-2 font-mono text-[11px]">
            SA
          </span>
          <span className="font-medium">Spotify Archivist</span>
        </header>
        <nav className="relative z-10 min-h-0 overflow-y-auto p-2">
          {sources.map((s) => {
            const Icon = s.kind === "liked_songs" ? Heart : ListMusic;
            const active = s.id === activeId;
            return (
              <Link
                key={s.id}
                to="/source/$id"
                params={{ id: String(s.id) }}
                className={cn(
                  "flex items-center gap-2 px-2 py-1.5 text-sm transition-colors duration-200 ease-out",
                  active
                    ? "bg-accent-soft text-accent"
                    : "text-muted hover:bg-surface-2 hover:text-fg",
                )}
              >
                <Icon size={14} className="shrink-0" />
                <span className="truncate">{s.name}</span>
              </Link>
            );
          })}
        </nav>
        <footer className="relative z-10 flex min-h-row items-center border-t border-border px-2">
          <Link
            to="/settings"
            className="flex flex-1 items-center gap-2 px-2 py-1.5 text-sm text-muted transition-colors duration-200 ease-out hover:bg-surface-2 hover:text-fg"
          >
            <SettingsIcon size={14} />
            <span>Settings</span>
          </Link>
        </footer>
      </aside>

      <main className="grid h-screen min-h-0 grid-rows-[var(--row-h)_1fr] overflow-hidden">
        <header className="hrow flex min-h-row items-center gap-4 border-b border-border bg-surface px-4">
          <div className="flex min-w-0 items-baseline gap-2">
            <h1 className="truncate font-medium">{sourceName}</h1>
            <span className="shrink-0 font-mono text-[11px] tabular-nums text-faint">
              {rows.length} rows
            </span>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <FilterPill value={filter} onChange={onFilter} />
            <button
              type="button"
              onClick={() => sync.mutate()}
              disabled={syncing}
              data-active={syncing ? "true" : undefined}
              className="pill flex items-center gap-1.5 px-3 py-1 text-xs"
            >
              <RefreshCcw size={12} className={cn("ic", syncing && "animate-spin")} />
              {syncing ? "Syncing…" : "Sync now"}
            </button>
            <div className="mx-1 h-5 w-px bg-border" aria-hidden="true" />
            <ThemeToggle />
            <PalettePopover />
          </div>
        </header>
        <TrackList rows={rows} isLoading={isLoading} isSyncing={syncing} />
      </main>
    </div>
  );
}
