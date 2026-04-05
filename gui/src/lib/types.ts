export interface ProxyStatus {
  running: boolean;
  listen_addr: string;
}

export interface StatsSnapshot {
  total: number;
  cache_hits: number;
  upstream: number;
  errors: number;
}

export type QueryStatus = "CacheHit" | "Upstream" | "Error";

export interface LogEntry {
  timestamp_unix: number;
  query_name: string;
  query_type: string;
  status: QueryStatus;
  latency_ms: number;
}

export interface CacheConfig {
  enabled: boolean;
  capacity: number;
}

export interface ProxyConfig {
  listen_addr: string;
  upstreams: string[];
  cache: CacheConfig;
}
