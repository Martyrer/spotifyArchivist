import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { PalettePopover } from "./PalettePopover";
import { useUi } from "@/store/useUi";

describe("PalettePopover", () => {
  it("toggles the menu open and closed via the trigger", () => {
    render(<PalettePopover />);
    const trigger = screen.getByLabelText("Palette");
    expect(screen.queryByRole("menu")).toBeNull();
    fireEvent.click(trigger);
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    fireEvent.click(trigger);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("selects a palette and closes the menu", () => {
    render(<PalettePopover />);
    fireEvent.click(screen.getByLabelText("Palette"));
    fireEvent.click(screen.getByText("Forest Moss"));
    expect(useUi.getState().palette).toBe("forest");
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("closes the menu on outside mousedown", () => {
    render(<PalettePopover />);
    fireEvent.click(screen.getByLabelText("Palette"));
    expect(screen.getByRole("menu")).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("menu")).toBeNull();
  });

  it("marks the active palette with aria-checked", () => {
    useUi.getState().setPalette("cool");
    render(<PalettePopover />);
    fireEvent.click(screen.getByLabelText("Palette"));
    const cool = screen.getByText("Cool Slate").closest("button");
    expect(cool?.getAttribute("aria-checked")).toBe("true");
  });
});
