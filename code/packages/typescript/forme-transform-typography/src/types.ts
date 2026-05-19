/**
 * types.ts — option bag for the typography transform.
 *
 * Every feature is independently toggleable so callers can
 * disable the rules they disagree with (e.g. a documentation
 * site that wants prose smart-quotes but NOT em-dashes from
 * `---`, since `---` is also a common Markdown thematic break
 * in source code samples).
 *
 * Defaults err on the side of "what HTML prose writers expect" —
 * smart quotes / dashes / ellipsis on by default, ligatures off
 * (the trademark-symbol substitutions are punchy in marketing
 * copy but surprising in technical writing).
 *
 * @module types
 */

/**
 * All flags default to `true` except `ligatures` (default
 * `false`).
 *
 *   - `smartQuotes` — straight `"` / `'` → typographic
 *     `"" '' ''`.
 *   - `dashes` — `--` → en dash (–); `---` → em dash (—).
 *     Order matters: the loop checks the longer pattern first.
 *   - `ellipsis` — `...` → `…` (U+2026).
 *   - `ligatures` — `(c)` → `©`, `(r)` → `®`, `(tm)` → `™`.
 *     Off by default because technical content uses `(c)` as a
 *     parenthetical c, not a copyright sign.
 */
export interface TypographyOptions {
  readonly smartQuotes?: boolean;
  readonly dashes?: boolean;
  readonly ellipsis?: boolean;
  readonly ligatures?: boolean;
}
