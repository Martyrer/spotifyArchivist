import { Monitor, Moon, Sun } from "lucide-react";
import { useUi } from "@/store/useUi";

const ICON = { light: Sun, dark: Moon, system: Monitor } as const;
const LABEL = { light: "Light", dark: "Dark", system: "System" } as const;

export function ThemeToggle() {
  const mode = useUi((s) => s.mode);
  const cycleMode = useUi((s) => s.cycleMode);
  const Icon = ICON[mode];
  return (
    <button
      type="button"
      onClick={cycleMode}
      title={`Theme: ${LABEL[mode]}`}
      aria-label={`Theme: ${LABEL[mode]}. Click to change.`}
      className="pill grid size-8 place-items-center"
    >
      <Icon className="ic size-4" />
    </button>
  );
}
