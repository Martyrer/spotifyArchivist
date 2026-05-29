import { createRoute, redirect } from "@tanstack/react-router";
import { rootRoute } from "./__root";
import { ipc } from "@/lib/ipc/client";

export const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  loader: async () => {
    const settings = await ipc.get_settings();
    if (!settings.authenticated) {
      throw redirect({ to: "/login" });
    }
    if (!settings.onboarded) {
      throw redirect({ to: "/onboarding" });
    }
    const sources = await ipc.list_sources();
    if (sources.length === 0) {
      throw redirect({ to: "/onboarding" });
    }
    throw redirect({ to: "/source/$id", params: { id: String(sources[0].id) } });
  },
  component: () => null,
});
