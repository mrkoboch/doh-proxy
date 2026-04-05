import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Config } from "./Config";
import * as api from "../lib/api";
import type { ProxyConfig } from "../lib/types";

vi.mock("../lib/api");

const defaultConfig: ProxyConfig = {
  listen_addr: "0.0.0.0:5353",
  upstreams: ["https://1.1.1.1/dns-query"],
  cache: { enabled: true, capacity: 10000 },
};

describe("Config", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.loadConfig).mockResolvedValue(defaultConfig);
  });

  it("loads and displays config on mount", async () => {
    render(<Config running={false} />);
    await waitFor(() =>
      expect(screen.getByDisplayValue("0.0.0.0:5353")).toBeInTheDocument()
    );
    expect(screen.getByDisplayValue("https://1.1.1.1/dns-query")).toBeInTheDocument();
  });

  it("shows warning banner when server is running", () => {
    render(<Config running={true} />);
    expect(screen.getByText(/stop the server/i)).toBeInTheDocument();
  });

  it("disables inputs when server is running", async () => {
    render(<Config running={true} />);
    await waitFor(() =>
      expect(screen.getByDisplayValue("0.0.0.0:5353")).toBeDisabled()
    );
  });

  it("calls saveConfig with updated values on Save", async () => {
    vi.mocked(api.saveConfig).mockResolvedValue(undefined);
    render(<Config running={false} />);
    await waitFor(() => screen.getByDisplayValue("0.0.0.0:5353"));

    fireEvent.change(screen.getByDisplayValue("0.0.0.0:5353"), {
      target: { value: "127.0.0.1:5353" },
    });
    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    await waitFor(() =>
      expect(api.saveConfig).toHaveBeenCalledWith(
        "127.0.0.1:5353",
        ["https://1.1.1.1/dns-query"],
        true,
        10000
      )
    );
  });

  it("shows success message after save", async () => {
    vi.mocked(api.saveConfig).mockResolvedValue(undefined);
    render(<Config running={false} />);
    await waitFor(() => screen.getByRole("button", { name: /save/i }));
    fireEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() =>
      expect(screen.getByText(/saved/i)).toBeInTheDocument()
    );
  });

  it("shows error message when saveConfig rejects", async () => {
    vi.mocked(api.saveConfig).mockRejectedValue(new Error("invalid listen address: not-valid"));
    render(<Config running={false} />);
    await waitFor(() => screen.getByRole("button", { name: /save/i }));
    fireEvent.click(screen.getByRole("button", { name: /save/i }));
    await waitFor(() =>
      expect(screen.getByText(/invalid listen address/i)).toBeInTheDocument()
    );
  });
});
