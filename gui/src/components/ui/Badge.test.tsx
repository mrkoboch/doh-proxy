import { render, screen } from "@testing-library/react";
import { Badge } from "./Badge";

describe("Badge", () => {
  it("renders success variant text", () => {
    render(<Badge variant="success">Cache Hit</Badge>);
    expect(screen.getByText("Cache Hit")).toBeInTheDocument();
  });

  it("renders error variant text", () => {
    render(<Badge variant="error">Error</Badge>);
    expect(screen.getByText("Error")).toBeInTheDocument();
  });

  it("renders info variant text", () => {
    render(<Badge variant="info">Upstream</Badge>);
    expect(screen.getByText("Upstream")).toBeInTheDocument();
  });

  it("renders neutral variant text", () => {
    render(<Badge variant="neutral">Unknown</Badge>);
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });
});
