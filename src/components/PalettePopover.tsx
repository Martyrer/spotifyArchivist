import { useEffect, useRef, useState } from "react";
import { Check, Palette as PaletteIcon } from "lucide-react";
import { useUi, type Palette } from "@/store/useUi";

const PALETTES: { id: Palette; name: string }[] = [
  { id: "warm", name: "Warm Sand" },
  { id: "cool", name: "Cool Slate" },
  { id: "forest", name: "Forest Moss" },
  { id: "graphite", name: "Graphite Cyan" },
];

export function PalettePopover() {
  const palette = useUi((s) => s.palette);
  const setPalette = useUi((s) => s.setPalette);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        title="Palette"
        aria-label="Palette"
        aria-haspopup="menu"
        aria-expanded={open}
        data-active={open ? "true" : undefined}
        className="pill grid size-8 place-items-center"
      >
        <PaletteIcon className="ic size-4" />
      </button>
      {open ? (
        <div
          role="menu"
          aria-label="Palette"
          className="absolute right-0 z-50 mt-1 w-48 border border-border bg-surface [box-shadow:var(--shadow)]"
        >
          {PALETTES.map((p) => {
            const active = palette === p.id;
            return (
              <button
                key={p.id}
                type="button"
                role="menuitemradio"
                aria-checked={active}
                onClick={() => {
                  setPalette(p.id);
                  setOpen(false);
                }}
                className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors duration-200 ease-out hover:bg-surface-2 aria-checked:text-accent"
              >
                <span
                  className="size-2.5 shrink-0 border border-border bg-current"
                  aria-hidden="true"
                />
                <span className="flex-1">{p.name}</span>
                {active ? <Check className="size-3.5" /> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
