/**
 * Hook to detect the user's motion preference.
 *
 * Checks the `prefers-reduced-motion: reduce` media query and
 * listens for changes. When reduced motion is preferred:
 *   - Particle animations should stop (set spawnRate to 0)
 *   - Show static indicators (arrows, labels) instead
 *   - CSS animations are already disabled via accessibility.css
 *
 * @returns true if the user prefers reduced motion
 */

import { useState, useEffect } from "react";

export function useReducedMotion(): boolean {
  const [prefersReduced, setPrefersReduced] = useState(() => {
    if (typeof window === "undefined") return false;
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  });

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handler = (e: MediaQueryListEvent) => setPrefersReduced(e.matches);
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, []);

  return prefersReduced;
}
