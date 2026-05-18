/**
 * registry.ts — `createThemeRegistry()` per FM04 §13.3.
 *
 * The theme registry is the lookup table the orchestrator consults
 * when a `StyleDocument` declares `theme: "<name>"`.  In v0 it's an
 * in-memory map; later (FM01 §3) a persistent registry may back it.
 *
 * ## API shape
 *
 *   const reg = createThemeRegistry();
 *   reg.register(brandTheme);          // keyed by brandTheme.name
 *   reg.register(highContrastTheme);
 *   reg.lookup("brand");               // → Theme | undefined
 *   reg.list();                        // → readonly ["brand", "high-contrast"]   (sorted)
 *
 * ## Semantics
 *
 * - **Replace-on-duplicate** — re-registering the same name overwrites.
 *   The use case is hot-reload during development: the dev server
 *   re-runs the theme producer on file change and the new value
 *   should win.  Production producers register once.
 * - **`list()` returns names in sorted order** — deterministic so
 *   downstream callers (config dumps, error messages, AOT manifests)
 *   are byte-stable across runs.
 * - **`lookup()` is read-only** — returns the registered `Theme` by
 *   reference (no defensive copy).  `Theme` is `readonly` throughout
 *   the IR, so structural mutation by callers is a type error.
 *
 * ## Security note — registry mutation safety
 *
 * The registry stores `Theme` values keyed by user-supplied names.
 * Two ways a malicious or buggy caller could try to misuse the map:
 *
 * 1. **Prototype-pollution names** — `"__proto__"` / `"constructor"` /
 *    `"prototype"` would, in a naive `{}`-backed map, poison
 *    `Object.prototype`.  We use a `Map<string, Theme>` rather than a
 *    plain object, so own-vs-inherited key semantics are not a
 *    concern; the `Map` API is closed over a discrete entry set.
 *    We additionally refuse these three names explicitly, throwing —
 *    they'd only ever appear by mistake.
 *
 * 2. **Reference aliasing** — the registry stores the `Theme` by
 *    reference.  Themes are `readonly` in the IR; TypeScript's
 *    `readonly` doesn't prevent runtime mutation, but it does flag
 *    accidents at compile time.  We choose not to deep-clone on
 *    register/lookup — the cost would be O(theme-size) per call and
 *    most callers never mutate.  The `readonly` discipline is the
 *    contract.
 *
 * @module registry
 */

import type { Theme } from "@coding-adventures/forme-style-ir";

/** Keys we refuse to register under — prototype-pollution shields. */
const FORBIDDEN_NAMES = new Set(["__proto__", "constructor", "prototype"]);

/** The public interface for the in-memory theme registry. */
export interface ThemeRegistry {
  /**
   * Insert (or replace) a theme.  Keyed by `theme.name`.  Throws if
   * `name` is empty or one of the prototype-pollution shield names
   * (which should never appear in real input but are refused
   * defensively).
   */
  register(theme: Theme): void;
  /** Lookup by name.  Returns `undefined` for unknown names. */
  lookup(name: string): Theme | undefined;
  /** All registered names, sorted lexicographically (deterministic). */
  list(): readonly string[];
}

/**
 * Construct a fresh, empty theme registry.  Each call yields an
 * independent registry — useful for tests and for orchestrators that
 * want isolated theme scopes (per-tenant, per-project, …).
 */
export function createThemeRegistry(): ThemeRegistry {
  // Backing `Map` rather than `{}` so we don't have to worry about
  // prototype-pollution semantics in the *storage*; we still refuse
  // the three forbidden names defensively to keep the registry's
  // observable surface clean.
  const themes = new Map<string, Theme>();

  return {
    register(theme: Theme): void {
      if (typeof theme.name !== "string" || theme.name.length === 0) {
        throw new Error("ThemeRegistry.register: theme.name must be a non-empty string");
      }
      if (FORBIDDEN_NAMES.has(theme.name)) {
        throw new Error(
          `ThemeRegistry.register: refusing forbidden name "${theme.name}"`,
        );
      }
      themes.set(theme.name, theme);
    },
    lookup(name: string): Theme | undefined {
      // `Map.get` is own-key lookup by construction — no inherited-
      // property concerns.
      return themes.get(name);
    },
    list(): readonly string[] {
      // Sort for byte-stable iteration.  Object.freeze stops accidental
      // in-place sorts by the caller from racing with future calls.
      return Object.freeze([...themes.keys()].sort());
    },
  };
}
