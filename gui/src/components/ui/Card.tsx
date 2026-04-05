import type { ReactNode } from "react";

interface Props {
  children: ReactNode;
  className?: string;
}

export function Card({ children, className = "" }: Props) {
  return (
    <div className={`bg-surface border border-border rounded-lg p-4 ${className}`}>
      {children}
    </div>
  );
}
