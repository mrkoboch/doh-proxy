interface Props {
  active: boolean;
  label?: string;
}

export function StatusDot({ active, label }: Props) {
  return (
    <div className="flex items-center gap-2">
      <span className="relative flex h-2.5 w-2.5">
        {active && (
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-50" />
        )}
        <span
          className={`relative inline-flex rounded-full h-2.5 w-2.5 ${
            active ? "bg-success" : "bg-muted"
          }`}
        />
      </span>
      {label && (
        <span
          className={`text-sm font-medium ${active ? "text-primary" : "text-secondary"}`}
        >
          {label}
        </span>
      )}
    </div>
  );
}
