import { invoke } from "@tauri-apps/api/core";
import type { LogEntry, ProxyConfig, ProxyStatus, StatsSnapshot } from "./types";

export function startProxy(): Promise<string> {
  return invoke<string>("start_proxy");
}

export function stopProxy(): Promise<void> {
  return invoke<void>("stop_proxy");
}

export function getProxyStatus(): Promise<ProxyStatus> {
  return invoke<ProxyStatus>("get_proxy_status");
}

export function getStats(): Promise<StatsSnapshot | null> {
  return invoke<StatsSnapshot | null>("get_stats");
}

export function getLogEntries(): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("get_log_entries");
}

export function loadConfig(): Promise<ProxyConfig> {
  return invoke<ProxyConfig>("load_config");
}

export function saveConfig(
  listenAddr: string,
  upstreams: string[],
  cacheEnabled: boolean,
  cacheCapacity: number
): Promise<void> {
  return invoke<void>("save_config", {
    listen_addr: listenAddr,
    upstreams,
    cache_enabled: cacheEnabled,
    cache_capacity: cacheCapacity,
  });
}
