import type { ReactNode } from "react";

interface Props {
  variant: "success" | "error" | "info" | "neutral";
  children: ReactNode;
}

export function Badge({ variant, children }: Props) {
  const variants = {
    success: "bg-success/10 text-success border-success/20",
    error:   "bg-error/10 text-error border-error/20",
    info:    "bg-info/10 text-info border-info/20",
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
