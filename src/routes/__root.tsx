import { Outlet, createRootRoute } from "@tanstack/react-router";
import { ThemeSync } from "@/components/ThemeSync";

export const rootRoute = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return (
    <div className="flex h-full w-full flex-col bg-bg text-fg font-sans text-[13px] leading-normal">
      <ThemeSync />
      <Outlet />
    </div>
  );
}
