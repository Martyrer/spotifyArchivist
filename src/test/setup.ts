import "@testing-library/jest-dom/vitest";

if (!("ResizeObserver" in globalThis)) {
  class StubResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  // happy-dom does not ship one; TanStack Virtual needs it.
  (globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver =
    StubResizeObserver;
}

