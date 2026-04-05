interface Props {
  variant: "success" | "error" | "info" | "neutral";
  children: React.ReactNode;
}

export function Badge({ variant, children }: Props) {
  const variants = {
    success: "bg-green-900/30 text-success border-green-800/40",
    error:   "bg-red-900/30 text-error border-red-800/40",
    info:    "bg-blue-900/30 text-info border-blue-800/40",
    neutral: "bg-elevated text-secondary border-border",
  };

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 text-xs font-medium rounded border ${variants[variant]}`}
    >
      {children}
    </span>
  );
}
