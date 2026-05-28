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
    <main className="mx-auto flex h-full w-full max-w-2xl flex-col gap-6 px-6 py-8">
      <header>
        <h1 className="text-xl font-semibold tracking-tight">Pick what to track</h1>
        <p className="text-sm text-neutral-400">
          Liked Songs is always tracked. Tick any of your own playlists you want the archivist
          to watch.
        </p>
      </header>

      <section className="rounded-lg border border-neutral-800 bg-neutral-900/50 px-4 py-3">
        <div className="flex items-center gap-3">
          <Heart size={18} className="text-emerald-400" />
          <div>
            <p className="text-sm font-medium">Liked Songs</p>
            <p className="text-xs text-neutral-500">Always on, cannot be disabled here.</p>
          </div>
        </div>
      </section>

      <section className="flex-1 overflow-y-auto rounded-lg border border-neutral-800">
        {isLoading ? (
          <div className="px-4 py-6 text-sm text-neutral-500">Loading playlists…</div>
        ) : playlists.length === 0 ? (
          <div className="px-4 py-6 text-sm text-neutral-500">No playlists found.</div>
        ) : (
          <ul>
            {playlists.map((p) => {
              const isPicked = picked.has(p.id) || p.already_tracked;
              return (
                <li key={p.id}>
                  <label
                    className={cn(
                      "flex cursor-pointer items-center gap-3 border-b border-neutral-800 px-4 py-2 last:border-b-0",
                      "hover:bg-neutral-800/40",
                    )}
                  >
                    <input
                      type="checkbox"
                      checked={isPicked}
                      disabled={p.already_tracked}
                      onChange={() => onTogglePick(p.id)}
                      className="size-4 accent-emerald-500"
                    />
                    <ListMusic size={16} className="text-neutral-500" />
                    <span className="text-sm">{p.name}</span>
                    {p.already_tracked ? (
                      <span className="ml-auto text-xs text-neutral-500">already tracked</span>
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
          className="rounded-full bg-emerald-500 px-5 py-2 text-sm font-medium text-neutral-950 transition hover:bg-emerald-400 disabled:opacity-50"
        >
          {isSubmitting ? "Starting first sync…" : "Continue"}
        </button>
      </footer>
    </main>
  );
}
