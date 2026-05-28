import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { LoginScreen } from "./LoginScreen";

describe("LoginScreen", () => {
  it("renders the call to action", () => {
    render(<LoginScreen onClick={() => undefined} isLoading={false} />);
    expect(screen.getByText("Login with Spotify")).toBeInTheDocument();
  });

  it("disables the button while loading and shows cancel", () => {
    const cancel = vi.fn();
    render(<LoginScreen onClick={() => undefined} onCancel={cancel} isLoading={true} />);
    const btn = screen.getByText("Waiting for browser…");
    expect(btn).toBeDisabled();
    fireEvent.click(screen.getByText("Cancel"));
    expect(cancel).toHaveBeenCalled();
  });

  it("renders an error message when provided", () => {
    render(
      <LoginScreen
        onClick={() => undefined}
        isLoading={false}
        error="boom"
      />,
    );
    expect(screen.getByText("boom")).toBeInTheDocument();
  });

  it("invokes onClick when pressed", () => {
    const spy = vi.fn();
    render(<LoginScreen onClick={spy} isLoading={false} />);
    fireEvent.click(screen.getByText("Login with Spotify"));
    expect(spy).toHaveBeenCalled();
  });
});
