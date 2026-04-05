import { describe, it, expect, vi, beforeEach } from "vitest";
import * as tauriCore from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core");

const { startProxy, stopProxy, getProxyStatus, getStats, getLogEntries, saveConfig } =
  await import("./api");

describe("api", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("startProxy invokes start_proxy and returns listen addr", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue("0.0.0.0:5353");
    const result = await startProxy();
    expect(tauriCore.invoke).toHaveBeenCalledWith("start_proxy");
    expect(result).toBe("0.0.0.0:5353");
  });

  it("stopProxy invokes stop_proxy", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(undefined);
    await stopProxy();
    expect(tauriCore.invoke).toHaveBeenCalledWith("stop_proxy");
  });

  it("getProxyStatus invokes get_proxy_status", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue({
      running: true,
      listen_addr: "0.0.0.0:5353",
    });
    const result = await getProxyStatus();
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_proxy_status");
    expect(result.running).toBe(true);
    expect(result.listen_addr).toBe("0.0.0.0:5353");
  });

  it("getStats invokes get_stats", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue({
      total: 42,
      cache_hits: 10,
      upstream: 30,
      errors: 2,
    });
    const result = await getStats();
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_stats");
    expect(result?.total).toBe(42);
  });

  it("getLogEntries invokes get_log_entries", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue([]);
    const result = await getLogEntries();
    expect(tauriCore.invoke).toHaveBeenCalledWith("get_log_entries");
    expect(Array.isArray(result)).toBe(true);
  });

  it("saveConfig passes snake_case args to invoke", async () => {
    vi.mocked(tauriCore.invoke).mockResolvedValue(undefined);
    await saveConfig("127.0.0.1:5353", ["https://1.1.1.1/dns-query"], true, 10000);
    expect(tauriCore.invoke).toHaveBeenCalledWith("save_config", {
      listen_addr: "127.0.0.1:5353",
      upstreams: ["https://1.1.1.1/dns-query"],
      cache_enabled: true,
      cache_capacity: 10000,
    });
  });
});
