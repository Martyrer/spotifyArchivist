import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { OnboardingScreen } from "./OnboardingScreen";

describe("OnboardingScreen", () => {
  it("loading state renders message", () => {
    render(
      <OnboardingScreen
        playlists={[]}
        isLoading={true}
        picked={new Set()}
        onTogglePick={() => undefined}
        onSubmit={() => undefined}
        isSubmitting={false}
      />,
    );
    expect(screen.getByText("Loading playlists…")).toBeInTheDocument();
  });

  it("empty list shows hint", () => {
    render(
      <OnboardingScreen
        playlists={[]}
        isLoading={false}
        picked={new Set()}
        onTogglePick={() => undefined}
        onSubmit={() => undefined}
        isSubmitting={false}
      />,
    );
    expect(screen.getByText("No playlists found.")).toBeInTheDocument();
  });

  it("renders playlists, marks already-tracked as locked, fires onTogglePick", () => {
    const toggle = vi.fn();
    render(
      <OnboardingScreen
        playlists={[
          { id: "p1", name: "Mix", already_tracked: false },
          { id: "p2", name: "Jazz", already_tracked: true },
        ]}
        isLoading={false}
        picked={new Set()}
        onTogglePick={toggle}
        onSubmit={() => undefined}
        isSubmitting={false}
      />,
    );
    const checkboxes = screen.getAllByRole("checkbox");
    expect(checkboxes).toHaveLength(2);
    expect(checkboxes[1]).toBeDisabled();
    fireEvent.click(checkboxes[0]);
    expect(toggle).toHaveBeenCalledWith("p1");
  });

  it("Continue button calls onSubmit", () => {
    const submit = vi.fn();
    render(
      <OnboardingScreen
        playlists={[]}
        isLoading={false}
        picked={new Set()}
        onTogglePick={() => undefined}
        onSubmit={submit}
        isSubmitting={false}
      />,
    );
    fireEvent.click(screen.getByText("Continue"));
    expect(submit).toHaveBeenCalled();
  });
});
