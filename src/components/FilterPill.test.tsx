import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FilterPill } from "./FilterPill";

describe("FilterPill", () => {
  it("renders all three options", () => {
    render(<FilterPill value="all" onChange={() => undefined} />);
    expect(screen.getByText("All")).toBeInTheDocument();
    expect(screen.getByText("Present")).toBeInTheDocument();
    expect(screen.getByText("Removed")).toBeInTheDocument();
  });

  it("marks active option via aria-checked", () => {
    render(<FilterPill value="removed" onChange={() => undefined} />);
    const removed = screen.getByText("Removed");
    expect(removed.getAttribute("aria-checked")).toBe("true");
    expect(screen.getByText("All").getAttribute("aria-checked")).toBe("false");
  });

  it("invokes onChange with the picked value", () => {
    const spy = vi.fn();
    render(<FilterPill value="all" onChange={spy} />);
    fireEvent.click(screen.getByText("Removed"));
    expect(spy).toHaveBeenCalledWith("removed");
  });
});
