import { Outlet, createRootRoute } from "@tanstack/react-router";

export const rootRoute = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return (
    <div className="flex h-full w-full flex-col bg-neutral-950 text-neutral-100">
      <Outlet />
    </div>
  );
}
