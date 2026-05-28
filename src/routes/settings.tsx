import { createRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { rootRoute } from "./__root";
import { ipc } from "@/lib/ipc/client";

export const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsRoute,
});

function SettingsRoute() {
  const qc = useQueryClient();
  const settings = useQuery({ queryKey: ["settings"], queryFn: ipc.get_settings });
  const sources = useQuery({ queryKey: ["sources"], queryFn: ipc.list_sources });
  const [hours, setHours] = useState<number | null>(null);
  const [scopeId, setScopeId] = useState<"all" | number>("all");

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

  const current = hours ?? settings.data?.sync_interval_hours ?? 6;

  return (
    <main className="mx-auto flex h-full w-full max-w-2xl flex-col gap-8 px-6 py-8">
      <header>
        <h1 className="text-xl font-semibold tracking-tight">Settings</h1>
      </header>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-neutral-300">Sync interval</h2>
        <div className="flex items-center gap-3">
          <input
            type="range"
            min={1}
            max={24}
            value={current}
            onChange={(e) => setHours(Number(e.target.value))}
            className="flex-1 accent-emerald-500"
          />
          <span className="w-16 text-sm tabular-nums">{current}h</span>
          <button
            type="button"
            disabled={update.isPending}
            onClick={() => update.mutate(current)}
            className="rounded-full bg-emerald-500 px-4 py-1 text-xs text-neutral-950 disabled:opacity-50"
          >
            Save
          </button>
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-neutral-300">Export JSONL</h2>
        <div className="flex items-center gap-3">
          <select
            value={scopeId}
            onChange={(e) =>
              setScopeId(e.target.value === "all" ? "all" : Number(e.target.value))
            }
            className="rounded-md border border-neutral-700 bg-neutral-900 px-3 py-1 text-sm"
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
            className="rounded-full border border-neutral-700 px-4 py-1 text-xs disabled:opacity-50"
          >
            {exportRun.isPending ? "Exporting…" : "Export"}
          </button>
          {exportRun.data ? (
            <span className="text-xs text-neutral-400">{exportRun.data} rows written</span>
          ) : null}
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-medium text-neutral-300">Account</h2>
        <button
          type="button"
          disabled={logout.isPending}
          onClick={() => logout.mutate()}
          className="rounded-full border border-rose-700 px-4 py-1 text-xs text-rose-300 disabled:opacity-50"
        >
          Log out
        </button>
        {settings.data?.user_id ? (
          <p className="text-xs text-neutral-500">Signed in as {settings.data.user_id}</p>
        ) : null}
      </section>
    </main>
  );
}
