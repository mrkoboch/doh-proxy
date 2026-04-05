type Page = "dashboard" | "config" | "stats" | "log";

const NAV: { page: Page; label: string; icon: string }[] = [
  { page: "dashboard", label: "Dashboard", icon: "◎" },
  { page: "config",    label: "Config",    icon: "⚙" },
  { page: "stats",     label: "Stats",     icon: "▦" },
  { page: "log",       label: "Query Log", icon: "≡" },
];

interface Props {
  current: Page;
  onNavigate: (page: Page) => void;
  running: boolean;
}

export function Sidebar({ current, onNavigate, running }: Props) {
  return (
    <aside className="w-44 flex-shrink-0 bg-surface border-r border-border flex flex-col select-none">
      <div className="px-4 py-5 border-b border-border">
        <span className="text-accent font-bold text-sm tracking-tight">DoH Proxy</span>
        <div className="mt-1.5 flex items-center gap-1.5">
          <span
            className={`h-1.5 w-1.5 rounded-full ${running ? "bg-success" : "bg-muted"}`}
          />
          <span className="text-xs text-secondary">
            {running ? "Running" : "Stopped"}
          </span>
        </div>
      </div>

      <nav className="flex-1 px-2 py-3 space-y-0.5">
        {NAV.map(({ page, label, icon }) => (
          <button
            key={page}
            onClick={() => onNavigate(page)}
            className={[
              "w-full flex items-center gap-3 px-3 py-2 rounded text-sm transition-colors text-left",
              current === page
                ? "bg-elevated text-primary font-medium"
                : "text-secondary hover:text-primary hover:bg-elevated/60",
            ].join(" ")}
          >
            <span className="font-mono text-xs w-4 text-center opacity-70">{icon}</span>
            {label}
          </button>
        ))}
      </nav>

      <div className="px-4 py-3 border-t border-border">
        <span className="text-xs text-muted">v0.1.3</span>
      </div>
    </aside>
  );
}

export type { Page };
