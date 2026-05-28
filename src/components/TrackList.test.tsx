import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { TrackList } from "./TrackList";

describe("TrackList", () => {
  it("shows loading state", () => {
    render(<TrackList rows={[]} isLoading={true} />);
    expect(screen.getByText("Loading…")).toBeInTheDocument();
  });

  it("shows empty hint when no rows", () => {
    render(<TrackList rows={[]} isLoading={false} />);
    expect(screen.getByText(/Nothing here yet/)).toBeInTheDocument();
  });
});
