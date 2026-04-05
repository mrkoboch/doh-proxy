import { useState, useEffect, useCallback } from "react";
import { Sidebar, type Page } from "./components/Sidebar";
import { getProxyStatus } from "./lib/api";
import type { ProxyStatus } from "./lib/types";
import { Dashboard } from "./pages/Dashboard";
import { Config }    from "./pages/Config";
import { Stats }     from "./pages/Stats";
import { QueryLog }  from "./pages/QueryLog";

export default function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const [status, setStatus] = useState<ProxyStatus>({
    running: false,
    listen_addr: "0.0.0.0:5353",
  });

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await getProxyStatus());
    } catch {
      // Tauri not ready yet — silently ignore
    }
  }, []);

  useEffect(() => {
    refreshStatus();
    const id = setInterval(refreshStatus, 1000);
    return () => clearInterval(id);
  }, [refreshStatus]);

  return (
    <div className="flex h-screen bg-base text-primary overflow-hidden">
      <Sidebar current={page} onNavigate={setPage} running={status.running} />
      <main className="flex-1 overflow-auto">
        {page === "dashboard" && (
          <Dashboard status={status} onStatusChange={refreshStatus} />
        )}
        {page === "config"    && <Config running={status.running} />}
        {page === "stats"     && <Stats />}
        {page === "log"       && <QueryLog />}
      </main>
    </div>
  );
}
