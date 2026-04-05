import { useState, useEffect } from "react";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { StatusDot } from "../components/ui/StatusDot";
import { startProxy, stopProxy, getStats } from "../lib/api";
import type { ProxyStatus, StatsSnapshot } from "../lib/types";

interface Props {
  status: ProxyStatus;
  onStatusChange: () => void;
}

export function Dashboard({ status, onStatusChange }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError]     = useState<string | null>(null);
  const [stats, setStats]     = useState<StatsSnapshot | null>(null);

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
  }, [status.running]);

  async function handleToggle() {
    setLoading(true);
    setError(null);
    try {
      if (status.running) {
        await stopProxy();
      } else {
        await startProxy();
      }
      await onStatusChange();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }

  const STAT_CARDS = stats
    ? [
        { label: "Total Queries", value: stats.total },
        { label: "Cache Hits",    value: stats.cache_hits },
        { label: "Upstream",      value: stats.upstream },
        { label: "Errors",        value: stats.errors },
      ]
    : [];

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-base font-semibold text-primary mb-5">Dashboard</h1>

      <Card className="mb-5">
        <div className="flex items-center justify-between">
          <div>
            <StatusDot active={status.running} label={status.running ? "Running" : "Stopped"} />
            <p className="mt-2 text-xs font-mono text-muted">{status.listen_addr}</p>
          </div>
          <Button
            variant={status.running ? "danger" : "primary"}
            onClick={handleToggle}
            disabled={loading}
          >
            {loading ? "…" : status.running ? "Stop" : "Start"}
          </Button>
        </div>
        {error && (
          <p className="mt-3 text-xs text-error border-t border-border/50 pt-3">{error}</p>
        )}
      </Card>

      {STAT_CARDS.length > 0 && (
        <div className="grid grid-cols-2 gap-3">
          {STAT_CARDS.map(({ label, value }) => (
            <Card key={label}>
              <p className="text-xs text-secondary uppercase tracking-wider mb-1.5">{label}</p>
              <p className="text-2xl font-mono font-semibold text-primary">
                {value.toLocaleString()}
              </p>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
