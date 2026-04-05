import { useState, useEffect } from "react";
import { Card } from "../components/ui/Card";
import { getStats } from "../lib/api";
import type { StatsSnapshot } from "../lib/types";

export function Stats() {
  const [stats, setStats] = useState<StatsSnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const s = await getStats();
        if (!cancelled) setStats(s);
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

  if (!stats) {
    return (
      <div className="p-6">
        <h1 className="text-base font-semibold text-primary mb-5">Statistics</h1>
        <p className="text-secondary text-sm">Start the server to see statistics.</p>
      </div>
    );
  }

  const pct = (n: number, d: number) =>
    d === 0 ? 0 : Math.min(100, Math.round((n / d) * 100));

  const hitRate = pct(stats.cache_hits, stats.total);
  const rows = [
    { label: "Total Queries",   value: stats.total,      pct: null,                              color: "text-primary"   },
    { label: "Cache Hits",      value: stats.cache_hits, pct: hitRate,                            color: "text-success"   },
    { label: "Upstream Queries",value: stats.upstream,   pct: pct(stats.upstream, stats.total),  color: "text-info"      },
    { label: "Errors",          value: stats.errors,     pct: pct(stats.errors, stats.total),    color: stats.errors > 0 ? "text-error" : "text-muted" },
  ] as const;

  return (
    <div className="p-6 max-w-xl">
      <h1 className="text-base font-semibold text-primary mb-5">Statistics</h1>

      <Card className="mb-4">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              {(["Metric", "Count", "%"] as const).map((h) => (
                <th
                  key={h}
                  className={`pb-2 text-xs text-secondary uppercase tracking-wider font-medium ${
                    h === "Metric" ? "text-left" : "text-right"
                  }`}
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-border/40">
            {rows.map(({ label, value, pct: p, color }) => (
              <tr key={label}>
                <td className="py-2.5 text-secondary text-sm">{label}</td>
                <td className={`py-2.5 text-right font-mono text-sm ${color}`}>
                  {value.toLocaleString()}
                </td>
                <td className="py-2.5 text-right font-mono text-sm text-secondary">
                  {p !== null ? `${p}%` : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {stats.total > 0 && (
        <Card>
          <p className="text-xs text-secondary uppercase tracking-wider mb-2">
            Cache Hit Rate
          </p>
          <div className="flex items-center gap-3">
            <div className="flex-1 bg-elevated rounded-full h-1.5">
              <div
                className="bg-success h-1.5 rounded-full transition-all duration-500"
                style={{ width: `${hitRate}%` }}
              />
            </div>
            <span className="text-sm font-mono text-success w-10 text-right">
              {hitRate}%
            </span>
          </div>
        </Card>
      )}
    </div>
  );
}
