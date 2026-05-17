/**
 * escape.ts — string sanitisation for terminal output.
 *
 * Two distinct concerns:
 *
 *   1. **ANSI escape-sequence injection.**  Anything we emit to a
 *      terminal that contains ESC (0x1b) or C1 CSI (0x9b) can move
 *      the cursor, clear the screen, change foreground colour for
 *      everything after, or — in extreme historical cases — execute
 *      keystrokes via DECSC/DECRC trickery.  We MUST strip these
 *      bytes from any caller-controlled string that lands in the
 *      output.
 *
 *   2. **TypeScript string-literal escaping.**  Our output is a TS
 *      module source string.  Caller-controlled data lands in
 *      double-quoted JS string literals (e.g. rule ids become Map
 *      keys).  A raw `"` or `\` in the input would terminate the
 *      literal early or alter the escape semantics of nearby bytes.
 *      We escape with the standard JS rules: `\\` first (so later
 *      `\"` escapes don't collide), then `\"`.
 *
 * Both passes also strip ASCII control characters (0x00–0x1F, 0x7F)
 * — they shouldn't appear in a legitimate rule id, and they're a
 * common vector for terminal-injection attacks.
 *
 * @module escape
 */

/**
 * The full set of bytes that can introduce, modify, or escape from a
 * terminal escape sequence.  Stripped unconditionally from every
 * caller-controlled string before further processing.
 *
 * Includes:
 *   - ESC (0x1B)        — introduces CSI / OSC / DCS / etc.
 *   - C1 CSI (0x9B)     — single-byte equivalent of `ESC [` in
 *                         8-bit-clean environments
 *   - All other ASCII control bytes (0x00–0x1F, 0x7F) — defence in
 *     depth; nothing legitimate has them
 *   - C1 controls (0x80–0x9F) — DCS (0x90), OSC (0x9D), ST (0x9C),
 *     and friends; not all terminals interpret them, but stripping
 *     is cheap and avoids assumptions
 *
 * The class is built explicitly to satisfy CodeQL's
 * incomplete-string-escaping rule (it wants to see "this regex
 * strips dangerous bytes" as a single, complete pass).
 */
// eslint-disable-next-line no-control-regex
const DANGEROUS_BYTES_RE = /[\x00-\x1F\x7F-\x9F]/g;

/**
 * Strip every byte that could introduce or alter an ANSI escape
 * sequence.  Used as the first pass before either text or
 * TS-string-literal escaping.
 */
export function stripAnsiUnsafe(s: string): string {
  return s.replace(DANGEROUS_BYTES_RE, "");
}

/**
 * Escape a string for use inside a double-quoted TypeScript string
 * literal in the generated module.  Strips ANSI-unsafe bytes first,
 * then escapes `\` and `"`.
 *
 * One-pass implementation via the
 * `[<class>]/g, (ch) => map[ch]` form — same pattern CodeQL accepts
 * without complaint.
 */
const TS_STRING_ESCAPE_MAP: Readonly<Record<string, string>> = Object.freeze({
  "\\": "\\\\",
  "\"": "\\\"",
});
// Match `\` or `"` — the two characters that need escaping inside a
// double-quoted JS/TS string.  Single pass, no order-dependent chain.
const TS_STRING_SPECIAL_RE = /[\\"]/g;

export function escapeTsString(s: string): string {
  return stripAnsiUnsafe(s).replace(TS_STRING_SPECIAL_RE, (ch) => TS_STRING_ESCAPE_MAP[ch]!);
}

/**
 * Sanitise a string for use as a Map key.  Same as `escapeTsString`
 * — different name documents the intent at the call site (keys vs.
 * values).
 */
export function sanitiseKey(s: string): string {
  return escapeTsString(s);
}
