// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect } from "react";

/** Apply theme ("light" | "dark" | "system") to the document root. */
export function useTheme(theme: string) {
  useEffect(() => {
    let resolvedTheme = theme;
    if (theme === "system") {
      resolvedTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
      const mql = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = (e: MediaQueryListEvent) => {
        document.documentElement.setAttribute("data-theme", e.matches ? "dark" : "light");
        document.body.style.background = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
      };
      mql.addEventListener("change", handler);
      document.documentElement.setAttribute("data-theme", resolvedTheme);
      document.body.style.background = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
      return () => mql.removeEventListener("change", handler);
    }
    document.documentElement.setAttribute("data-theme", resolvedTheme);
    document.body.style.background = getComputedStyle(document.documentElement).getPropertyValue("--bg").trim();
  }, [theme]);
}
