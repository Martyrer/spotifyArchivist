import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "light" | "dark";
export type Palette = "warm" | "cool" | "forest" | "graphite";

interface UiState {
  theme: Theme;
  palette: Palette;
  setTheme: (t: Theme) => void;
  toggleTheme: () => void;
  setPalette: (p: Palette) => void;
}

export const useUi = create<UiState>()(
  persist(
    (set) => ({
      theme: "dark",
      palette: "warm",
      setTheme: (theme) => set({ theme }),
      toggleTheme: () => set((s) => ({ theme: s.theme === "dark" ? "light" : "dark" })),
      setPalette: (palette) => set({ palette }),
    }),
    { name: "forge-ui" },
  ),
);
