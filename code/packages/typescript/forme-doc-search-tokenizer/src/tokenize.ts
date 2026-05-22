/**
 * tokenize.ts — main `tokenize` entry composing the pipeline.
 *
 * @module tokenize
 */

import { normaliseToTokens } from "./normalise.js";
import { porterStem } from "./porter.js";
import { STOP_WORDS } from "./stop-words.js";
import type { TokenizeOptions } from "./types.js";

/**
 * Tokenise `text` per the DOC00 spec pipeline:
 *
 *   1. Lowercase (locale-independent).
 *   2. Strip non-alphanumeric (keep Unicode letters/digits +
 *      underscore).
 *   3. Split on whitespace / punctuation runs.
 *   4. Filter stop-words (when `options.filterStopWords`).
 *   5. Porter stem each surviving token (when `options.stem`).
 *
 * The four behaviour flags (`filterStopWords`, `stem`,
 * `customStopWords`) are all optional; the default is
 * `tokenize(text)` → just steps 1-3 (lowercased
 * alphanumeric tokens, no filtering, no stemming).
 *
 * @param text - The input string.  Non-string inputs are
 *               coerced via `String(...)`.
 * @param options - `{ filterStopWords?, stem?, customStopWords? }`.
 * @returns A flat list of tokens in source order.
 */
export function tokenize(text: string, options: TokenizeOptions = {}): string[] {
  const tokens = normaliseToTokens(String(text));
  // Phase 4: stop-word filter.
  let filtered = tokens;
  if (options.filterStopWords === true) {
    const stopWords = options.customStopWords ?? STOP_WORDS;
    filtered = tokens.filter((t) => !stopWords.has(t));
  }
  // Phase 5: Porter stem.
  if (options.stem === true) {
    filtered = filtered.map(porterStem);
  }
  return filtered;
}
