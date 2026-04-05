import { useState, useEffect } from "react";
import { Badge } from "../components/ui/Badge";
import { getLogEntries } from "../lib/api";
import type { LogEntry, QueryStatus } from "../lib/types";

function formatTime(unix: number): string {
  const d = new Date(unix * 1000);
  return [d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds()]
    .map((n) => String(n).padStart(2, "0"))
    .join(":") + " UTC";
}

function statusBadgeVariant(s: QueryStatus): "success" | "info" | "error" {
  if (s === "CacheHit") return "success";
  if (s === "Upstream") return "info";
  return "error";
}

function statusLabel(s: QueryStatus): string {
  if (s === "CacheHit") return "Cache Hit";
  if (s === "Upstream") return "Upstream";
  return "Error";
}

export function QueryLog() {
  const [entries, setEntries] = useState<LogEntry[]>([]);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const e = await getLogEntries();
        if (!cancelled) setEntries(e);
      } catch {
        // proxy not running
      }
    };
    poll();
    const id = setInterval(poll, 1000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <div className="p-6 flex flex-col h-full">
      <div className="flex items-center justify-between mb-5">
        <h1 className="text-base font-semibold text-primary">Query Log</h1>
        {entries.length > 0 && (
          <span className="text-xs text-muted">{entries.length} entries</span>
        )}
      </div>

      {entries.length === 0 ? (
        <p className="text-secondary text-sm italic">
          No queries yet. Start the server and route DNS traffic through it.
        </p>
      ) : (
        <div className="flex-1 min-h-0 overflow-auto border border-border rounded-lg">
          <table className="w-full text-xs">
            <thead className="sticky top-0 bg-surface border-b border-border z-10">
              <tr>
                {["Time", "Query Name", "Type", "Status", "Latency"].map((h) => (
                  <th
                    key={h}
                    className="py-2.5 px-3 text-left text-secondary uppercase tracking-wider font-medium"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="bg-base divide-y divide-border/30">
              {entries.map((entry, i) => (
                <tr
                  key={i}
                  className={`${i % 2 === 1 ? "bg-surface/30" : ""} hover:bg-elevated/50 transition-colors`}
                >
                  <td className="py-2 px-3 font-mono text-muted whitespace-nowrap">
                    {formatTime(entry.timestamp_unix)}
                  </td>
                  <td className="py-2 px-3 font-mono text-primary max-w-xs truncate">
                    {entry.query_name}
                  </td>
                  <td className="py-2 px-3 font-mono text-secondary">{entry.query_type}</td>
                  <td className="py-2 px-3">
                    <Badge variant={statusBadgeVariant(entry.status)}>
                      {statusLabel(entry.status)}
                    </Badge>
                  </td>
                  <td className="py-2 px-3 font-mono text-secondary">
                    {entry.latency_ms}ms
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
