import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { TrackList } from "./TrackList";

describe("TrackList", () => {
  it("shows loading state", () => {
    render(<TrackList rows={[]} isLoading={true} isSyncing={false} />);
    expect(screen.getByText("Loading…")).toBeInTheDocument();
  });

  it("shows empty hint when no rows", () => {
    render(<TrackList rows={[]} isLoading={false} isSyncing={false} />);
    expect(screen.getByText(/Nothing here yet/)).toBeInTheDocument();
  });

  it("shows syncing indicator when syncing with no rows", () => {
    render(<TrackList rows={[]} isLoading={false} isSyncing={true} />);
    expect(screen.getByText("Syncing…")).toBeInTheDocument();
  });
});
