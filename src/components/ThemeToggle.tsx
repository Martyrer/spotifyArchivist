import { Moon, Sun } from "lucide-react";
import { useUi } from "@/store/useUi";

export function ThemeToggle() {
  const theme = useUi((s) => s.theme);
  const toggleTheme = useUi((s) => s.toggleTheme);
  return (
    <button
      type="button"
      onClick={toggleTheme}
      title="Toggle theme"
      aria-label="Toggle theme"
      className="pill grid size-8 place-items-center"
    >
      {theme === "dark" ? <Sun className="ic size-4" /> : <Moon className="ic size-4" />}
    </button>
  );
}
