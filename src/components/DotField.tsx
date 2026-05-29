import { useEffect, useRef } from "react";
import { cn } from "@/lib/cn";

type Props = {
  className?: string;
  /** Cell size in px (grid resolution). */
  cell?: number;
};

/**
 * Conway's Game of Life as a subtle themed background, modelled on forgecode's
 * hero animation. Living cells render as small --accent squares at low alpha
 * with a 0.03/frame fade trail; the cursor seeds new life in a 5x5 patch and
 * carries a soft radial glow. Colours read live from CSS vars, re-theming on
 * data-theme / data-palette swaps. Pin behind content with absolute inset-0;
 * aria-hidden + non-interactive.
 */
export function DotField({ className, cell = 12 }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // --life is oklch(). getComputedStyle may return it verbatim (not rgb),
    // and canvas rejects oklch as fillStyle. Resolve to true rgb by painting
    // the colour onto a 1x1 scratch canvas and reading the pixel back — the
    // engine does the colour-space conversion.
    const probe = document.createElement("span");
    probe.style.cssText = "position:absolute;width:0;height:0;pointer-events:none;";
    canvas.parentElement?.appendChild(probe);
    const scratch = document.createElement("canvas");
    scratch.width = scratch.height = 1;
    const sctx = scratch.getContext("2d", { willReadFrequently: true });
    let accent: [number, number, number] = [251, 146, 60];
    const readColors = () => {
      probe.style.color = "var(--life)";
      const resolved = getComputedStyle(probe).color; // e.g. "oklch(...)" or "rgb(...)"
      if (!sctx) return;
      sctx.clearRect(0, 0, 1, 1);
      sctx.fillStyle = "#000";
      sctx.fillStyle = resolved; // ignored if invalid → stays #000, harmless
      sctx.fillRect(0, 0, 1, 1);
      const [r, g, b] = sctx.getImageData(0, 0, 1, 1).data;
      // Guard: if the engine rejected the colour it stays #000 — keep prior.
      if (r || g || b) accent = [r, g, b];
    };

    // ── Grid state ─────────────────────────────────────────────────────────
    let cols = 0;
    let rows = 0;
    let grid = new Uint8Array(0); // 1 = alive
    let next = new Uint8Array(0);
    let disp = new Float32Array(0); // display intensity 0..1 (fade trail)
    let w = 0;
    let h = 0;
    let dpr = 1;

    const allocate = () => {
      cols = Math.ceil(w / cell);
      rows = Math.ceil(h / cell);
      const len = cols * rows;
      grid = new Uint8Array(len);
      next = new Uint8Array(len);
      disp = new Float32Array(len);
      for (let i = 0; i < len; i++) {
        grid[i] = Math.random() > 0.92 ? 1 : 0; // ~8% seed density
        disp[i] = grid[i];
      }
    };

    const resize = () => {
      const parent = canvas.parentElement;
      if (!parent) return;
      const rect = parent.getBoundingClientRect();
      w = rect.width;
      h = rect.height;
      dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      allocate();
    };

    // Pointer in CSS px; -9999 when off-canvas.
    const pointer = { x: -9999, y: -9999 };
    const onPointerMove = (e: PointerEvent) => {
      const rect = canvas.getBoundingClientRect();
      pointer.x = e.clientX - rect.left;
      pointer.y = e.clientY - rect.top;
    };
    const onPointerLeave = () => {
      pointer.x = -9999;
      pointer.y = -9999;
    };

    // Cursor seeds a 5x5 patch of new life each frame → living wake.
    const seedAtPointer = () => {
      if (pointer.x < -9000) return;
      const cx = Math.floor(pointer.x / cell);
      const cy = Math.floor(pointer.y / cell);
      for (let dx = -2; dx <= 2; dx++) {
        for (let dy = -2; dy <= 2; dy++) {
          const x = cx + dx;
          const y = cy + dy;
          if (x < 0 || y < 0 || x >= cols || y >= rows) continue;
          if (Math.random() > 0.7) {
            const i = y * cols + x;
            grid[i] = 1;
            disp[i] = 1;
          }
        }
      }
    };

    // One Conway generation (B3/S23) with toroidal wrap, into `next`.
    const step = () => {
      for (let y = 0; y < rows; y++) {
        const yUp = ((y - 1 + rows) % rows) * cols;
        const yMid = y * cols;
        const yDn = ((y + 1) % rows) * cols;
        for (let x = 0; x < cols; x++) {
          const xL = (x - 1 + cols) % cols;
          const xR = (x + 1) % cols;
          const n =
            grid[yUp + xL] +
            grid[yUp + x] +
            grid[yUp + xR] +
            grid[yMid + xL] +
            grid[yMid + xR] +
            grid[yDn + xL] +
            grid[yDn + x] +
            grid[yDn + xR];
          const i = yMid + x;
          next[i] = grid[i] ? (n === 2 || n === 3 ? 1 : 0) : n === 3 ? 1 : 0;
        }
      }
      const tmp = grid;
      grid = next;
      next = tmp;
    };

    const draw = () => {
      ctx.clearRect(0, 0, w, h);

      // Ambient background wash — faint accent pool, top-left biased.
      const [ar, ag, ab] = accent;
      const bg = ctx.createRadialGradient(
        w * 0.3,
        h * 0.4,
        0,
        w * 0.3,
        h * 0.4,
        w * 0.8,
      );
      bg.addColorStop(0, `rgba(${ar},${ag},${ab},0.01)`);
      bg.addColorStop(1, "transparent");
      ctx.fillStyle = bg;
      ctx.fillRect(0, 0, w, h);

      // Cells: living = full intensity, dead = fade 0.03/frame. 11px square.
      const sq = cell - 1;
      for (let y = 0; y < rows; y++) {
        for (let x = 0; x < cols; x++) {
          const i = y * cols + x;
          if (grid[i]) disp[i] = 1;
          else disp[i] = Math.max(0, disp[i] - 0.02);
          if (disp[i] > 0) {
            ctx.fillStyle = `rgba(${ar},${ag},${ab},${(0.08 * disp[i]).toFixed(3)})`;
            ctx.fillRect(x * cell, y * cell, sq, sq);
          }
        }
      }

      // Cursor glow — soft 120px accent halo while pointer is over the field.
      if (pointer.x > -9000) {
        const g = ctx.createRadialGradient(
          pointer.x,
          pointer.y,
          0,
          pointer.x,
          pointer.y,
          120,
        );
        g.addColorStop(0, `rgba(${ar},${ag},${ab},0.04)`);
        g.addColorStop(1, "transparent");
        ctx.fillStyle = g;
        ctx.fillRect(0, 0, w, h);
      }
    };

    // ── Loop: render every frame, step the sim every 100ms (~10 gen/s) ──────
    let raf = 0;
    let frameCount = 0;
    let lastStep = 0;
    const loop = (time: number) => {
      if (frameCount % 40 === 0) readColors();
      frameCount++;
      seedAtPointer();
      if (time - lastStep > 200) {
        step();
        lastStep = time;
      }
      draw();
      raf = requestAnimationFrame(loop);
    };

    readColors();
    resize();

    const ro = new ResizeObserver(resize);
    if (canvas.parentElement) ro.observe(canvas.parentElement);
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerleave", onPointerLeave);

    raf = requestAnimationFrame(loop);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      probe.remove();
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerleave", onPointerLeave);
    };
  }, [cell]);

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      className={cn("pointer-events-none absolute inset-0 z-0", className)}
    />
  );
}
