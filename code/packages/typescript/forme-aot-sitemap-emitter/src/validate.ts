/**
 * validate.ts — changefreq allowlist + priority clamping.
 *
 * Two small validators that turn caller-supplied scalar fields
 * into emit-safe values.  Both run BEFORE any XML interpolation
 * so the emitted document always conforms to the sitemap
 * protocol.
 *
 * @module validate
 */

import type { ChangeFreq } from "./types.js";

const CHANGEFREQ_ALLOWLIST: ReadonlySet<string> = new Set([
  "always",
  "hourly",
  "daily",
  "weekly",
  "monthly",
  "yearly",
  "never",
]);

/**
 * Validate `changefreq` against the sitemap protocol allowlist.
 * Returns the lowercased value on success; throws `TypeError`
 * otherwise.
 *
 * Lowercase comparison so callers can pass `"Daily"` or
 * `"DAILY"` without surprises.
 */
export function validateChangefreq(value: string): ChangeFreq {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-sitemap-emitter: changefreq must be a string; got ${typeof value}`,
    );
  }
  const lower = value.toLowerCase();
  if (!CHANGEFREQ_ALLOWLIST.has(lower)) {
    throw new TypeError(
      `forme-aot-sitemap-emitter: changefreq must be one of [${
        [...CHANGEFREQ_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return lower as ChangeFreq;
}

/**
 * Clamp `priority` to the sitemap-protocol range `[0.0, 1.0]`
 * and format with exactly one decimal place for byte-
 * deterministic output.
 *
 *   priority 0    → "0.0"
 *   priority 0.75 → "0.8"  (toFixed(1) rounds half-to-even)
 *   priority 1    → "1.0"
 *   priority 5    → "1.0"  (clamped)
 *   priority -1   → "0.0"  (clamped)
 *   priority NaN  → "0.5"  (sentinel — spec default is 0.5)
 *
 * NaN treated as 0.5 (the protocol's documented default for
 * "no priority specified" — keeps the slot non-empty rather
 * than synthesising an opinion).
 */
export function clampPriority(value: number): string {
  if (typeof value !== "number" || Number.isNaN(value)) return "0.5";
  if (value <= 0) return "0.0";
  if (value >= 1) return "1.0";
  return value.toFixed(1);
}
