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

  useEffect(() => {
    const unTray = listen("sync:trigger-from-tray", () => {
      void ipc.trigger_sync();
    });
    const unDone = listen("sync:completed", () => {
      qc.invalidateQueries({ queryKey: ["memberships"] });
      qc.invalidateQueries({ queryKey: ["sources"] });
    });
    return () => {
      void unTray.then((u) => u());
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
      filter={filter}
      onFilter={setFilter}
    />
  );
}
