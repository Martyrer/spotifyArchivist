import { useEffect } from "react";
import { useUi } from "@/store/useUi";

export function ThemeSync() {
  const mode = useUi((s) => s.mode);
  const palette = useUi((s) => s.palette);

  useEffect(() => {
    const r = document.documentElement;
    r.dataset.palette = palette;

    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      r.dataset.theme = mode === "system" ? (mql.matches ? "dark" : "light") : mode;
    };
    apply();

    // Only track the OS while following it.
    if (mode === "system") {
      mql.addEventListener("change", apply);
      return () => mql.removeEventListener("change", apply);
    }
  }, [mode, palette]);

  return null;
}
