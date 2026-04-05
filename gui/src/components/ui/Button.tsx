import { type ButtonHTMLAttributes } from "react";

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "danger" | "ghost";
  size?: "sm" | "md";
}

export function Button({
  variant = "primary",
  size = "md",
  className = "",
  children,
  ...props
}: Props) {
  const base =
    "inline-flex items-center justify-center rounded font-medium transition-colors " +
    "focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-base " +
    "disabled:opacity-40 disabled:cursor-not-allowed";

  const variants = {
    primary: "bg-accent text-base hover:bg-accent/85 focus:ring-accent",
    danger:  "bg-error text-base hover:bg-error/85 focus:ring-error",
    ghost:   "bg-transparent text-secondary hover:text-primary hover:bg-elevated focus:ring-border",
  };

  const sizes = {
    sm: "px-3 py-1.5 text-xs",
    md: "px-4 py-2 text-sm",
  };

  return (
    <button
      className={`${base} ${variants[variant]} ${sizes[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
