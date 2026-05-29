import type { AvailablePlaylist } from "@/lib/ipc/types";
import { cn } from "@/lib/cn";
import { Heart, ListMusic } from "lucide-react";

type Props = {
  playlists: AvailablePlaylist[];
  isLoading: boolean;
  picked: Set<string>;
  onTogglePick: (id: string) => void;
  onSubmit: () => void;
  isSubmitting: boolean;
};

export function OnboardingScreen({
  playlists,
  isLoading,
  picked,
  onTogglePick,
  onSubmit,
  isSubmitting,
}: Props) {
  return (
    <main className="mx-auto flex h-full w-full max-w-2xl flex-col gap-6 bg-bg px-6 py-8 text-fg">
      <header className="space-y-1">
        <h1 className="text-lg font-semibold tracking-tight">Pick what to track</h1>
        <p className="text-sm text-muted">
          Liked Songs is always tracked. Tick any of your own playlists you want the archivist
          to watch.
        </p>
      </header>

      <section className="fc flex items-center gap-3 border border-border bg-surface px-4 py-3">
        <Heart size={18} className="text-accent" />
        <div>
          <p className="text-sm font-medium">Liked Songs</p>
          <p className="text-xs text-faint">Always on, cannot be disabled here.</p>
        </div>
      </section>

      <section className="fc flex-1 overflow-y-auto border border-border bg-surface">
        {isLoading ? (
          <div className="px-4 py-6 text-sm text-muted">Loading playlists…</div>
        ) : playlists.length === 0 ? (
          <div className="px-4 py-6 text-sm text-muted">No playlists found.</div>
        ) : (
          <ul>
            {playlists.map((p) => {
              const isPicked = picked.has(p.id) || p.already_tracked;
              return (
                <li key={p.id}>
                  <label
                    className={cn(
                      "flex cursor-pointer items-center gap-3 border-b border-border-2 px-4 py-2 last:border-b-0",
                      "transition-colors duration-200 ease-out hover:bg-surface-2",
                    )}
                  >
                    <input
                      type="checkbox"
                      checked={isPicked}
                      disabled={p.already_tracked}
                      onChange={() => onTogglePick(p.id)}
                      className="size-4 accent-[var(--accent)]"
                    />
                    <ListMusic size={16} className="text-muted" />
                    <span className="text-sm">{p.name}</span>
                    {p.already_tracked ? (
                      <span className="ml-auto font-mono text-[11px] text-faint">already tracked</span>
                    ) : null}
                  </label>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      <footer className="flex justify-end">
        <button
          type="button"
          onClick={onSubmit}
          disabled={isSubmitting}
          data-active={isSubmitting ? "true" : undefined}
          className="pill px-5 py-2 text-sm font-medium"
        >
          {isSubmitting ? "Starting first sync…" : "Continue"}
        </button>
      </footer>
    </main>
  );
}
