import { useEffect } from "react";
import { useUi } from "@/store/useUi";

export function ThemeSync() {
  const theme = useUi((s) => s.theme);
  const palette = useUi((s) => s.palette);
  useEffect(() => {
    const r = document.documentElement;
    r.dataset.theme = theme;
    r.dataset.palette = palette;
  }, [theme, palette]);
  return null;
}
