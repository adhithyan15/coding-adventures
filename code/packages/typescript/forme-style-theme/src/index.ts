/**
 * @coding-adventures/forme-style-theme
 *
 * Theme registry + composition for Forme Style IR (FM04 §7, §13.3).
 *
 * Three exports, three concerns:
 *
 *   composeWithTheme(base, theme) → StyleDocument
 *     Apply a `Theme`'s sparse token overrides + appended rules to a
 *     base `StyleDocument`, returning a new merged document.  Per
 *     FM04 §7.2.
 *
 *   createThemeRegistry() → ThemeRegistry
 *     In-memory map { register, lookup, list } keyed by theme name.
 *     Dev-mode hot-reload friendly (re-register replaces).  Per
 *     FM04 §13.3.
 *
 *   resolveTokenRefs(doc, refs) → Map<string, ResolvedValue | null>
 *     Bulk `TokenRef` resolution against a document's tokens.  For
 *     analyser pre-passes (AOT CSS slicer, theme coverage report,
 *     etc.).  Per FM04 §3.5.
 *
 * This package has **no I/O** and **no shell** — pure
 * data-in / data-out plus an in-memory map.
 *
 * @module index
 */

export { composeWithTheme } from "./compose.js";
export { createThemeRegistry } from "./registry.js";
export type { ThemeRegistry } from "./registry.js";
export { resolveTokenRefs } from "./resolve.js";
export type { ResolvedValue } from "./resolve.js";
