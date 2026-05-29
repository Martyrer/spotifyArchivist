import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ConfirmDialog } from "./ConfirmDialog";

const base = {
  title: "Reset?",
  body: "This wipes everything.",
  confirmLabel: "Reset",
  onConfirm: () => undefined,
  onCancel: () => undefined,
};

describe("ConfirmDialog", () => {
  it("renders nothing when closed", () => {
    const { container } = render(<ConfirmDialog {...base} open={false} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders title and body when open", () => {
    render(<ConfirmDialog {...base} open />);
    expect(screen.getByText("Reset?")).toBeInTheDocument();
    expect(screen.getByText("This wipes everything.")).toBeInTheDocument();
  });

  it("fires onConfirm and onCancel on button clicks", () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(<ConfirmDialog {...base} open onConfirm={onConfirm} onCancel={onCancel} />);
    fireEvent.click(screen.getByText("Reset"));
    fireEvent.click(screen.getByText("Cancel"));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("cancels on Escape", () => {
    const onCancel = vi.fn();
    render(<ConfirmDialog {...base} open onCancel={onCancel} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("cancels on overlay mousedown", () => {
    const onCancel = vi.fn();
    render(<ConfirmDialog {...base} open onCancel={onCancel} />);
    fireEvent.mouseDown(screen.getByRole("dialog"));
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("disables buttons and ignores Escape while busy", () => {
    const onCancel = vi.fn();
    render(<ConfirmDialog {...base} open busy onCancel={onCancel} />);
    expect(screen.getByText("Cancel")).toBeDisabled();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onCancel).not.toHaveBeenCalled();
  });
});
