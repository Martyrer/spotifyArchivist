import { create } from "zustand";
import { persist } from "zustand/middleware";

export type Theme = "light" | "dark";
export type ThemeMode = Theme | "system";
export type Palette = "warm" | "cool" | "forest" | "graphite";

const MODE_CYCLE: ThemeMode[] = ["light", "dark", "system"];

interface UiState {
  mode: ThemeMode;
  palette: Palette;
  setMode: (m: ThemeMode) => void;
  cycleMode: () => void;
  setPalette: (p: Palette) => void;
}

export const useUi = create<UiState>()(
  persist(
    (set) => ({
      mode: "system",
      palette: "warm",
      setMode: (mode) => set({ mode }),
      cycleMode: () =>
        set((s) => ({ mode: MODE_CYCLE[(MODE_CYCLE.indexOf(s.mode) + 1) % MODE_CYCLE.length] })),
      setPalette: (palette) => set({ palette }),
    }),
    {
      name: "forge-ui",
      version: 1,
      // v0 persisted `theme: "light" | "dark"`; carry it over as the mode.
      migrate: (state, version) => {
        const s = state as Partial<UiState> & { theme?: Theme };
        if (version === 0 && s.theme) return { ...s, mode: s.theme };
        return s as UiState;
      },
    },
  ),
);
