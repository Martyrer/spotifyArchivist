import { Link, createRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { ArrowLeft, Trash2 } from "lucide-react";
import { rootRoute } from "./__root";
import { ipc } from "@/lib/ipc/client";
import { formatLastSync } from "@/lib/formatLastSync";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { DotField } from "@/components/DotField";

export const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsRoute,
});

function SettingsRoute() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  const settings = useQuery({ queryKey: ["settings"], queryFn: ipc.get_settings });
  const [confirmReset, setConfirmReset] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const t = e.target as HTMLElement | null;
      // Let Esc close a native control (e.g. open <select>) before leaving.
      if (t && /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName)) return;
      // The dialog owns Esc while it's open.
      if (confirmReset) return;
      navigate({ to: "/" });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, confirmReset]);
  const sources = useQuery({ queryKey: ["sources"], queryFn: ipc.list_sources });
  const [hours, setHours] = useState<number | null>(null);
  const [scopeId, setScopeId] = useState<"all" | number>("all");

  useEffect(() => {
    const unDone = listen("sync:completed", () => {
      qc.invalidateQueries({ queryKey: ["settings"] });
    });
    return () => {
      void unDone.then((u) => u());
    };
  }, [qc]);

  const update = useMutation({
    mutationFn: (h: number) => ipc.update_settings(h),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["settings"] }),
  });
  const exportRun = useMutation({
    mutationFn: async () => {
      const path = await save({
        defaultPath: "spotify-archivist.jsonl",
        filters: [{ name: "JSONL", extensions: ["jsonl"] }],
      });
      if (!path) return 0;
      const scope = scopeId === "all" ? { kind: "all" as const } : { kind: "source" as const, id: scopeId };
      return ipc.export(scope, path);
    },
  });
  const logout = useMutation({
    mutationFn: ipc.logout,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["settings"] }),
  });
  const untrack = useMutation({
    mutationFn: (id: number) => ipc.untrack_source(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["sources"] }),
  });
  const reset = useMutation({
    mutationFn: ipc.reset_app,
    onSuccess: async () => {
      setConfirmReset(false);
      await qc.invalidateQueries();
      navigate({ to: "/" });
    },
  });

  if (!settings.isSuccess) {
    return null;
  }
  const current = hours ?? settings.data.sync_interval_hours;
  const lastSync = formatLastSync(settings.data.last_sync_at);

  return (
    <div className="relative h-screen w-full overflow-hidden bg-bg text-fg">
      <DotField />
      <main className="relative z-10 h-full w-full overflow-x-hidden overflow-y-auto">
      <header className="hrow flex min-h-row items-center gap-3 border-b border-border bg-surface px-4">
        <Link
          to="/"
          className="pill grid size-8 place-items-center"
          aria-label="Back"
        >
          <ArrowLeft size={14} />
        </Link>
        <h1 className="font-medium">Settings</h1>
      </header>

      <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-6 py-8">
      <section className="space-y-2">
        <h2 className="font-mono text-[11px] uppercase tracking-[0.06em] text-faint">
          Sync interval
        </h2>
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={1}
            max={24}
            value={current}
            onChange={(e) => setHours(Number(e.target.value))}
            onPointerUp={(e) => update.mutate(Number(e.currentTarget.value))}
            onKeyUp={(e) => update.mutate(Number(e.currentTarget.value))}
            className="flex-1 accent-[var(--accent)]"
          />
          <span className="min-w-[2.5rem] text-right font-mono text-sm tabular-nums">{current}h</span>
        </div>
        <p className="font-mono text-xs tabular-nums text-faint">
          Last sync: <span className="text-muted">{lastSync}</span>
        </p>
      </section>

      <section className="space-y-2">
        <h2 className="font-mono text-[11px] uppercase tracking-[0.06em] text-faint">
          Export JSONL
        </h2>
        <div className="flex items-center gap-3">
          <select
            value={scopeId}
            onChange={(e) =>
              setScopeId(e.target.value === "all" ? "all" : Number(e.target.value))
            }
            className="rounded-control border border-border bg-surface px-3 py-1 text-sm"
          >
            <option value="all">All sources</option>
            {(sources.data ?? []).map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          <button
            type="button"
            disabled={exportRun.isPending}
            onClick={() => exportRun.mutate()}
            data-active={exportRun.isPending ? "true" : undefined}
            className="pill px-4 py-1 text-xs"
          >
            {exportRun.isPending ? "Exporting…" : "Export"}
          </button>
          {exportRun.data ? (
            <span className="font-mono text-xs tabular-nums text-muted">
              {exportRun.data} rows written
            </span>
          ) : null}
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="font-mono text-[11px] uppercase tracking-[0.06em] text-faint">
          Tracked playlists
        </h2>
        <ul className="fc border border-border bg-surface">
          {(sources.data ?? []).map((s) => {
            const locked = s.kind === "liked_songs";
            return (
              <li
                key={s.id}
                className="flex items-center justify-between border-b border-border-2 px-4 py-2 last:border-b-0"
              >
                <span className="text-sm">{s.name}</span>
                {locked ? (
                  <span className="font-mono text-[11px] text-faint">always tracked</span>
                ) : (
                  <button
                    type="button"
                    onClick={() => untrack.mutate(s.id)}
                    disabled={untrack.isPending}
                    aria-label={`Stop tracking ${s.name}`}
                    className="pill grid size-7 place-items-center"
                  >
                    <Trash2 size={14} className="ic" />
                  </button>
                )}
              </li>
            );
          })}
          {(sources.data ?? []).length === 0 ? (
            <li className="px-4 py-3 text-sm text-muted">No sources yet.</li>
          ) : null}
        </ul>
      </section>

      <section className="space-y-2">
        <h2 className="font-mono text-[11px] uppercase tracking-[0.06em] text-faint">
          Account
        </h2>
        <button
          type="button"
          disabled={logout.isPending}
          onClick={() => logout.mutate()}
          data-active={logout.isPending ? "true" : undefined}
          className="pill px-4 py-1 text-xs"
        >
          Log out
        </button>
        {settings.data.user_id ? (
          <p className="font-mono text-xs text-faint">Signed in as {settings.data.user_id}</p>
        ) : null}
      </section>

      <section className="space-y-2">
        <h2 className="font-mono text-[11px] uppercase tracking-[0.06em] text-faint">
          Danger zone
        </h2>
        <div className="fc flex items-center justify-between gap-4 border border-border bg-surface px-4 py-3">
          <div>
            <p className="text-sm font-medium">Reset application</p>
            <p className="text-xs text-muted">
              Wipes every tracked song, playlist, and sync record, signs you out, and returns the
              app to its first-run state.
            </p>
          </div>
          <button
            type="button"
            onClick={() => setConfirmReset(true)}
            className="pill pill-danger shrink-0 px-4 py-1 text-xs"
          >
            Reset
          </button>
        </div>
      </section>
      </div>

      <ConfirmDialog
        open={confirmReset}
        title="Reset application?"
        confirmLabel={reset.isPending ? "Resetting…" : "Reset everything"}
        busy={reset.isPending}
        danger
        onConfirm={() => reset.mutate()}
        onCancel={() => setConfirmReset(false)}
        body={
          <>
            This permanently deletes all tracked sources, songs, and sync history from the local
            database, clears your Spotify credentials, and resets all settings to their defaults.
            <br />
            <br />
            This cannot be undone. You will need to log in and pick what to track again.
          </>
        }
      />
      </main>
    </div>
  );
}
