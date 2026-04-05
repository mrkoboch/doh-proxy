import { useState, useEffect } from "react";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { loadConfig, saveConfig } from "../lib/api";
import type { ProxyConfig } from "../lib/types";

interface Props {
  running: boolean;
}

export function Config({ running }: Props) {
  const [config, setConfig]             = useState<ProxyConfig | null>(null);
  const [listenAddr, setListenAddr]     = useState("");
  const [upstreams, setUpstreams]       = useState<string[]>([]);
  const [cacheEnabled, setCacheEnabled] = useState(true);
  const [cacheCapacity, setCacheCapacity] = useState("10000");
  const [saving, setSaving]             = useState(false);
  const [msg, setMsg] = useState<{ type: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    loadConfig().then((cfg) => {
      setConfig(cfg);
      setListenAddr(cfg.listen_addr);
      setUpstreams(cfg.upstreams);
      setCacheEnabled(cfg.cache.enabled);
      setCacheCapacity(String(cfg.cache.capacity));
    });
  }, []);

  const disabled = running || !config;

  async function handleSave() {
    setSaving(true);
    setMsg(null);
    try {
      await saveConfig(listenAddr, upstreams, cacheEnabled, Number(cacheCapacity));
      setMsg({ type: "success", text: "Configuration saved." });
    } catch (e) {
      setMsg({ type: "error", text: e instanceof Error ? e.message : String(e) });
    } finally {
      setSaving(false);
    }
  }

  function addUpstream() {
    setUpstreams((prev) => [...prev, "https://"]);
  }
  function removeUpstream(i: number) {
    setUpstreams((prev) => prev.filter((_, idx) => idx !== i));
  }
  function updateUpstream(i: number, val: string) {
    setUpstreams((prev) => prev.map((u, idx) => (idx === i ? val : u)));
  }

  return (
    <div className="p-6 max-w-xl">
      <h1 className="text-base font-semibold text-primary mb-5">Configuration</h1>

      {running && (
        <div className="mb-4 px-4 py-2.5 bg-warning/10 border border-warning/20 rounded text-xs text-warning">
          Stop the server before editing configuration.
        </div>
      )}

      <Card className="space-y-5">
        {/* Listen address */}
        <div>
          <label className="block text-xs text-secondary uppercase tracking-wider mb-1.5">
            Listen Address
          </label>
          <input
            value={listenAddr}
            onChange={(e) => setListenAddr(e.target.value)}
            disabled={disabled}
            className="w-full bg-elevated border border-border rounded px-3 py-2 text-sm font-mono text-primary
                       focus:outline-none focus:ring-1 focus:ring-accent disabled:opacity-40"
          />
        </div>

        {/* Upstreams */}
        <div>
          <label className="block text-xs text-secondary uppercase tracking-wider mb-1.5">
            Upstream DNS-over-HTTPS URLs
          </label>
          <div className="space-y-2">
            {upstreams.map((url, i) => (
              <div key={i} className="flex gap-2">
                <input
                  value={url}
                  onChange={(e) => updateUpstream(i, e.target.value)}
                  disabled={disabled}
                  className="flex-1 bg-elevated border border-border rounded px-3 py-2 text-sm font-mono text-primary
                             focus:outline-none focus:ring-1 focus:ring-accent disabled:opacity-40"
                />
                {!disabled && (
                  <button
                    onClick={() => removeUpstream(i)}
                    className="px-2 text-muted hover:text-error transition-colors text-sm"
                  >
                    ✕
                  </button>
                )}
              </div>
            ))}
          </div>
          {!disabled && (
            <button
              onClick={addUpstream}
              className="mt-2 text-xs text-accent hover:text-orange-400 transition-colors"
            >
              + Add upstream
            </button>
          )}
        </div>

        {/* Cache */}
        <div className="flex items-center gap-6">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={cacheEnabled}
              onChange={(e) => setCacheEnabled(e.target.checked)}
              disabled={disabled}
            />
            <span className="text-sm text-secondary">Cache enabled</span>
          </label>
          <div className="flex items-center gap-2">
            <span className="text-sm text-secondary">Capacity</span>
            <input
              value={cacheCapacity}
              onChange={(e) => setCacheCapacity(e.target.value)}
              disabled={disabled}
              className="w-24 bg-elevated border border-border rounded px-2 py-1.5 text-sm font-mono text-primary
                         focus:outline-none focus:ring-1 focus:ring-accent disabled:opacity-40"
            />
          </div>
        </div>

        {/* Save row */}
        <div className="flex items-center justify-between pt-1 border-t border-border/50">
          <Button onClick={handleSave} disabled={disabled || saving}>
            {saving ? "Saving…" : "Save Config"}
          </Button>
          {msg && (
            <p className={`text-xs ${msg.type === "success" ? "text-success" : "text-error"}`}>
              {msg.text}
            </p>
          )}
        </div>
      </Card>
    </div>
  );
}
