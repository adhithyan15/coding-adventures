/**
 * strings.ts — Type-safe i18n string accessor.
 *
 * All user-visible text in the app lives in `strings.en.json`. This module
 * provides a typed accessor `t(key, params?)` that:
 *
 *   1. Validates the key at compile time (TypeScript will error on typos).
 *   2. Performs optional `{key}` → value substitution for dynamic content.
 *   3. Makes locale switching a one-line change (swap the catalog reference).
 *
 * === Usage ===
 *
 *   import { t } from "../strings.js";
 *
 *   // Static string
 *   <h2>{t("task.form.headingNew")}</h2>
 *
 *   // Interpolated string
 *   t("task.form.history.timeMinutes", { n: 5 })   // → "5m ago"
 *   t("task.form.history.eventUpdatedFields", { fields: "title, priority" })
 *     // → "Updated: title, priority"
 *
 * === Locale switching ===
 *
 * Today the app ships English only. To add French:
 *   1. Create `strings.fr.json` with the same key structure, French values.
 *   2. Change the `catalog` assignment below to `fr` based on user preference.
 *
 * The `t()` function and all call sites remain unchanged.
 *
 * === NestedKeys<T> ===
 *
 * This recursive TypeScript type produces the union of all valid dot-path
 * strings for a deeply nested object. Example:
 *
 *   const obj = { a: { b: "hello", c: "world" }, d: "!" };
 *   type Keys = NestedKeys<typeof obj>;
 *   // → "a.b" | "a.c" | "d"
 *
 * This means TypeScript catches key typos at compile time:
 *
 *   t("task.form.headingNwe")  // ← TypeScript error: not a valid key
 *   t("task.form.headingNew")  // ← OK
 */

import en from "./strings.en.json";

// ── Types ─────────────────────────────────────────────────────────────────────

/**
 * NestedKeys<T> — produces a union of all dot-path keys in a nested object
 * whose leaf values are strings.
 *
 * Example:
 *   type K = NestedKeys<{ a: { b: "x"; c: "y" }; d: "z" }>;
 *   // → "a.b" | "a.c" | "d"
 *
 * How it works:
 *   For each key K in T:
 *     - If T[K] is a string → emit "Prefix.K" (the leaf)
 *     - If T[K] is an object → recurse with prefix "Prefix.K."
 *     - Otherwise → never (skip non-string, non-object values)
 */
export type NestedKeys<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string
    ? `${Prefix}${K}`
    : T[K] extends Record<string, unknown>
      ? NestedKeys<T[K], `${Prefix}${K}.`>
      : never;
}[keyof T & string];

// ── Catalog ───────────────────────────────────────────────────────────────────

/**
 * The active locale catalog. Swapping this reference changes the entire
 * app's language without touching any component.
 *
 * `typeof en` is the structural type — this ensures `catalog` must have
 * exactly the same shape as the English file, so a French file with missing
 * keys is a type error.
 */
const catalog: typeof en = en;

// ── Internal helpers ──────────────────────────────────────────────────────────

/**
 * getNestedValue — traverse a dot-path into a nested object and return the
 * leaf string.
 *
 * Falls back to the raw key if the path doesn't resolve (which should never
 * happen in production since NestedKeys<T> prevents invalid keys).
 */
function getNestedValue(obj: Record<string, unknown>, path: string): string {
  const parts = path.split(".");
  let current: unknown = obj;

  for (const part of parts) {
    if (typeof current !== "object" || current === null) return path;
    current = (current as Record<string, unknown>)[part];
  }

  return typeof current === "string" ? current : path;
}

// ── Public API ────────────────────────────────────────────────────────────────

/**
 * t — translate a key to the current locale's string, with optional params.
 *
 * @param key    — dot-path into the catalog, e.g. "task.form.headingNew"
 * @param params — substitution map for {placeholder} values in the string
 *
 * Interpolation examples:
 *
 *   t("task.form.history.timeMinutes", { n: 3 })
 *   // "3m ago"
 *
 *   t("task.form.history.eventUpdatedFields", { fields: "title, priority" })
 *   // "Updated: title, priority"
 *
 *   t("task.form.history.eventStatusSet", { status: "done" })
 *   // "Status set to done"
 *
 * Missing params: if a placeholder has no corresponding param, it is left
 * as-is (e.g., `{n}`) rather than crashing. This prevents a bad call from
 * breaking the UI.
 */
export function t(
  key: NestedKeys<typeof en>,
  params?: Record<string, string | number>,
): string {
  const raw = getNestedValue(catalog as Record<string, unknown>, key);
  if (!params) return raw;

  // Replace every {placeholder} with the corresponding param value.
  // If the param is missing, leave the placeholder as-is.
  return raw.replace(/\{(\w+)\}/g, (match, k: string) =>
    k in params ? String(params[k]) : match,
  );
}
