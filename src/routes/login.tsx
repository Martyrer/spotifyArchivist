import { createRoute, useNavigate } from "@tanstack/react-router";
import { useMutation } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { rootRoute } from "./__root";
import { ipc } from "@/lib/ipc/client";
import { LoginScreen } from "@/components/LoginScreen";

export const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  component: LoginRoute,
});

function LoginRoute() {
  const navigate = useNavigate();
  const login = useMutation({
    mutationFn: async () => {
      const { authorize_url } = await ipc.start_login();
      await openUrl(authorize_url);
      return await ipc.await_login();
    },
    onSuccess: () => navigate({ to: "/" }),
  });
  return (
    <LoginScreen
      onClick={() => login.mutate()}
      onCancel={() => {
        ipc.cancel_login().catch(() => undefined);
        login.reset();
      }}
      isLoading={login.isPending}
      error={login.error?.message}
    />
  );
}
