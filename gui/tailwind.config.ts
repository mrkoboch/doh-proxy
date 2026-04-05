import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        base:     "#0e1219",
        surface:  "#151e2a",
        elevated: "#1c2837",
        border:   "#273547",
        accent:   "#38c9c0",
        success:  "#34d399",
        error:    "#f87171",
        info:     "#60a5fa",
        warning:  "#f4956a",
        primary:   "#e4eaf3",
        secondary: "#7a8fa3",
        muted:     "#3d5168",
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
} satisfies Config;
