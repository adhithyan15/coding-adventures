/**
 * useCanvasTheme — bridges CSS custom properties into Canvas drawing code.
 *
 * The Canvas 2D API has no concept of CSS. It draws with explicit color
 * strings (`ctx.fillStyle = "#ff0000"`). But our theme is defined as CSS
 * custom properties in theme.css (e.g., `--panel-bg: #16213e`). We need a
 * bridge between these two worlds.
 *
 * === How It Works ===
 *
 * ```
 * Container <div> mounts in the DOM
 *   → useEffect fires (runs after paint, so computed styles are available)
 *     → getComputedStyle(container) reads live CSS values
 *       → Returns typed CanvasTheme object
 *         → Canvas useEffect draws with these colors
 * ```
 *
 * The hook reads from a ref'd DOM element rather than `document.documentElement`
 * so that scoped themes (e.g., a light-mode panel inside a dark-mode page)
 * work correctly. The element's computed styles reflect its actual CSS context.
 *
 * === Fallback Values ===
 *
 * Each property has a hardcoded fallback matching the default dark theme. If
 * the CSS custom properties are not defined (e.g., theme.css was not imported),
 * the canvas still renders with sensible dark colors.
 */

import { useEffect, useState, type RefObject } from "react";

/**
 * Theme values extracted from CSS custom properties for Canvas rendering.
 *
 * Each field maps to one or more CSS custom properties from theme.css.
 * The Canvas rendering pipeline uses these values for fill colors, stroke
 * colors, and font selection.
 */
export interface CanvasTheme {
  /** Background color for the table body area. From --panel-bg. */
  bodyBg: string;
  /** Text color for body cells. From --body-text or color. */
  bodyText: string;
  /** Background for the header row. From --panel-header or --tab-active. */
  headerBg: string;
  /** Text color for header cells. From --tab-active-text. */
  headerText: string;
  /** Color for grid lines (borders). From --panel-border. */
  borderColor: string;
  /** Alternating row background (slightly lighter than bodyBg). */
  altRowBg: string;
  /** Sans-serif font family. From --sans. */
  fontFamily: string;
}

/** Dark theme defaults matching theme.css values. */
const DEFAULTS: CanvasTheme = {
  bodyBg: "#16213e",
  bodyText: "#e0e0e0",
  headerBg: "#0f3460",
  headerText: "#ffffff",
  borderColor: "#2a3a5e",
  altRowBg: "rgba(255, 255, 255, 0.02)",
  fontFamily: "system-ui, -apple-system, sans-serif",
};

/**
 * Reads a CSS custom property from an element's computed style.
 * Returns the trimmed value, or the fallback if the property is empty/unset.
 */
function readCssProp(
  style: CSSStyleDeclaration,
  prop: string,
  fallback: string,
): string {
  const value = style.getPropertyValue(prop).trim();
  return value || fallback;
}

/**
 * Hook that reads CSS custom properties from a mounted DOM element and
 * returns them as a typed CanvasTheme object.
 *
 * The theme is read once on mount. Future versions can add a MutationObserver
 * on the `<html>` element to detect runtime theme class changes.
 */
export function useCanvasTheme(
  containerRef: RefObject<HTMLElement | null>,
): CanvasTheme {
  const [theme, setTheme] = useState<CanvasTheme>(DEFAULTS);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const style = getComputedStyle(el);

    setTheme({
      bodyBg: readCssProp(style, "--panel-bg", DEFAULTS.bodyBg),
      bodyText: readCssProp(style, "--body-text", DEFAULTS.bodyText) ||
        readCssProp(style, "color", DEFAULTS.bodyText),
      headerBg: readCssProp(style, "--panel-header", DEFAULTS.headerBg) ||
        readCssProp(style, "--tab-active", DEFAULTS.headerBg),
      headerText: readCssProp(style, "--tab-active-text", DEFAULTS.headerText),
      borderColor: readCssProp(style, "--panel-border", DEFAULTS.borderColor),
      altRowBg: DEFAULTS.altRowBg,
      fontFamily: readCssProp(style, "--sans", DEFAULTS.fontFamily),
    });
  }, [containerRef]);

  return theme;
}
