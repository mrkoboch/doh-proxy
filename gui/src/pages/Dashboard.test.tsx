import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Dashboard } from "./Dashboard";
import * as api from "../lib/api";
import type { ProxyStatus } from "../lib/types";

vi.mock("../lib/api");

const stoppedStatus: ProxyStatus = { running: false, listen_addr: "0.0.0.0:5353" };
const runningStatus: ProxyStatus = { running: true,  listen_addr: "0.0.0.0:5353" };

describe("Dashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.getStats).mockResolvedValue(null);
  });

  it("shows Start button when stopped", () => {
    render(<Dashboard status={stoppedStatus} onStatusChange={() => {}} />);
    expect(screen.getByRole("button", { name: /^start$/i })).toBeInTheDocument();
  });

  it("shows Stop button when running", () => {
    render(<Dashboard status={runningStatus} onStatusChange={() => {}} />);
    expect(screen.getByRole("button", { name: /^stop$/i })).toBeInTheDocument();
  });

  it("calls startProxy and onStatusChange when Start is clicked", async () => {
    vi.mocked(api.startProxy).mockResolvedValue("0.0.0.0:5353");
    const onStatusChange = vi.fn();
    render(<Dashboard status={stoppedStatus} onStatusChange={onStatusChange} />);
    fireEvent.click(screen.getByRole("button", { name: /^start$/i }));
    await waitFor(() => expect(api.startProxy).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onStatusChange).toHaveBeenCalledTimes(1));
  });

  it("calls stopProxy and onStatusChange when Stop is clicked", async () => {
    vi.mocked(api.stopProxy).mockResolvedValue(undefined);
    const onStatusChange = vi.fn();
    render(<Dashboard status={runningStatus} onStatusChange={onStatusChange} />);
    fireEvent.click(screen.getByRole("button", { name: /^stop$/i }));
    await waitFor(() => expect(api.stopProxy).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onStatusChange).toHaveBeenCalledTimes(1));
  });

  it("shows error message when startProxy rejects", async () => {
    vi.mocked(api.startProxy).mockRejectedValue(new Error("port in use"));
    render(<Dashboard status={stoppedStatus} onStatusChange={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /^start$/i }));
    await waitFor(() =>
      expect(screen.getByText(/port in use/i)).toBeInTheDocument()
    );
  });

  it("renders listen address", () => {
    render(<Dashboard status={stoppedStatus} onStatusChange={() => {}} />);
    expect(screen.getByText("0.0.0.0:5353")).toBeInTheDocument();
  });

  it("renders stats cards when stats are available", async () => {
    vi.mocked(api.getStats).mockResolvedValue({
      total: 100, cache_hits: 60, upstream: 38, errors: 2,
    });
    render(<Dashboard status={runningStatus} onStatusChange={() => {}} />);
    await waitFor(() => expect(screen.getByText("100")).toBeInTheDocument());
    expect(screen.getByText("60")).toBeInTheDocument();
  });
});
