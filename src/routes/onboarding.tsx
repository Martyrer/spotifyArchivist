import { createRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { rootRoute } from "./__root";
import { ipc } from "@/lib/ipc/client";
import { OnboardingScreen } from "@/components/OnboardingScreen";

export const onboardingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/onboarding",
  component: OnboardingRoute,
});

function OnboardingRoute() {
  const navigate = useNavigate();
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const playlists = useQuery({
    queryKey: ["available-playlists"],
    queryFn: ipc.list_available_playlists,
  });
  const submit = useMutation({
    mutationFn: async () => {
      const list = playlists.data ?? [];
      for (const p of list) {
        if (picked.has(p.id) && !p.already_tracked) {
          await ipc.track_playlist(p.id, p.name);
        }
      }
      const sources = await ipc.list_sources();
      void ipc.trigger_sync();
      const liked = sources.find((s) => s.kind === "liked_songs");
      return liked?.id ?? sources[0]?.id;
    },
    onSuccess: (id) => {
      if (id !== undefined) {
        navigate({ to: "/source/$id", params: { id: String(id) } });
      }
    },
  });
  return (
    <OnboardingScreen
      playlists={playlists.data ?? []}
      isLoading={playlists.isLoading}
      picked={picked}
      onTogglePick={(id) => {
        setPicked((prev) => {
          const next = new Set(prev);
          if (next.has(id)) next.delete(id);
          else next.add(id);
          return next;
        });
      }}
      onSubmit={() => submit.mutate()}
      isSubmitting={submit.isPending}
    />
  );
}
