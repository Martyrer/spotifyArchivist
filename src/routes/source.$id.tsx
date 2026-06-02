import { createRoute } from "@tanstack/react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { rootRoute } from "./__root";
import { ipc } from "@/lib/ipc/client";
import type { MembershipFilter } from "@/lib/ipc/types";
import { SourceShell } from "@/components/SourceShell";

export const sourceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/source/$id",
  component: SourceRoute,
});

function SourceRoute() {
  const { id } = sourceRoute.useParams();
  const sourceId = Number(id);
  const [filter, setFilter] = useState<MembershipFilter>("all");
  const qc = useQueryClient();

  useEffect(() => {
    ipc.mark_seen().catch(() => undefined);
  }, [sourceId]);

  const [isSyncing, setIsSyncing] = useState(false);

  // Reflect a sync that is already running when this view mounts (e.g. the
  // startup auto-sync, or navigating in mid-sync) — events only fire on edges.
  useEffect(() => {
    ipc.get_sync_status().then(setIsSyncing).catch(() => undefined);
  }, []);

  useEffect(() => {
    const unTray = listen("sync:trigger-from-tray", () => {
      void ipc.trigger_sync();
    });
    const unStarted = listen("sync:started", () => setIsSyncing(true));
    const unDone = listen("sync:completed", () => {
      setIsSyncing(false);
      qc.invalidateQueries({ queryKey: ["memberships"] });
      qc.invalidateQueries({ queryKey: ["sources"] });
      qc.invalidateQueries({ queryKey: ["settings"] });
    });
    return () => {
      void unTray.then((u) => u());
      void unStarted.then((u) => u());
      void unDone.then((u) => u());
    };
  }, [qc]);

  const sources = useQuery({
    queryKey: ["sources"],
    queryFn: ipc.list_sources,
  });
  const rows = useQuery({
    queryKey: ["memberships", sourceId, filter],
    queryFn: () => ipc.list_memberships(sourceId, filter),
    enabled: !Number.isNaN(sourceId),
  });

  const source = sources.data?.find((s) => s.id === sourceId);
  return (
    <SourceShell
      sources={sources.data ?? []}
      activeId={sourceId}
      sourceName={source?.name ?? "Loading…"}
      rows={rows.data ?? []}
      isLoading={rows.isLoading}
      isSyncing={isSyncing}
      filter={filter}
      onFilter={setFilter}
    />
  );
}
