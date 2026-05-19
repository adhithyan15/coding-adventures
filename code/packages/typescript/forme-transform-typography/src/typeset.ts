/**
 * typeset.ts — single-pass character loop that applies typography
 * substitutions to a string.
 *
 * Why a character loop and not regex `.replace`?
 *
 *   - **Zero ReDoS surface.**  Patterns like `/-{2,3}/g` with
 *     `String.prototype.replace` are linear in modern engines,
 *     but CodeQL's polynomial-regex heuristic flags them and
 *     other static analysers similarly worry.  A single forward
 *     `for` loop is unambiguously O(n) with no backtracking.
 *   - **Lookahead semantics are explicit.**  Em dash vs en dash
 *     vs hyphen depends on the next 1-2 chars.  Open vs close
 *     quote depends on the previous char's class.  Coding these
 *     branches with `charCodeAt(i+k)` is more obvious than a
 *     stack of regex alternatives.
 *   - **One pass, not five.**  The naive chained-`.replace`
 *     version walks the string once per rule.  This implementation
 *     walks it exactly once total.
 *
 * Substitutions (in lookahead-precedence order — checked left to
 * right within each character):
 *
 * | Source                     | Output    | Codepoint |
 * |----------------------------|-----------|-----------|
 * | `---`                      | em dash   | U+2014    |
 * | `--`                       | en dash   | U+2013    |
 * | `...`                      | ellipsis  | U+2026    |
 * | `"` (after WS / start)     | left-DQ   | U+201C    |
 * | `"` (otherwise)            | right-DQ  | U+201D    |
 * | `'` (after alphanumeric)   | right-SQ  | U+2019    |
 * |                              (apostrophe — `don't`, `it's`) |
 * | `'` (after WS / start)     | left-SQ   | U+2018    |
 * | `'` (otherwise)            | right-SQ  | U+2019    |
 * | `(c)` / `(C)` (ligatures)  | copyright | U+00A9    |
 * | `(r)` / `(R)` (ligatures)  | registered| U+00AE    |
 * | `(tm)` / `(TM)` (ligatures)| trademark | U+2122    |
 *
 * **All other characters pass through verbatim.**  Including
 * existing typographic characters (callers can run this on
 * already-prettified text; it's idempotent for the substitution
 * subset).
 *
 * @module typeset
 */

import type { TypographyOptions } from "./types.js";

// Character codes (named for readability in the hot loop).
const CC_QUOTE_DOUBLE = 34;  // "
const CC_QUOTE_SINGLE = 39;  // '
const CC_HYPHEN = 45;        // -
const CC_DOT = 46;           // .
const CC_LPAREN = 40;        // (
const CC_RPAREN = 41;        // )
const CC_LOWER_C = 99;
const CC_UPPER_C = 67;
const CC_LOWER_R = 114;
const CC_UPPER_R = 82;
const CC_LOWER_T = 116;
const CC_UPPER_T = 84;
const CC_LOWER_M = 109;
const CC_UPPER_M = 77;

// Output strings (compile-time constants, no allocation in the loop).
const EM_DASH = "—";
const EN_DASH = "–";
const ELLIPSIS = "…";
const LEFT_DQ = "“";
const RIGHT_DQ = "”";
const LEFT_SQ = "‘";
const RIGHT_SQ = "’";
const COPYRIGHT = "©";
const REGISTERED = "®";
const TRADEMARK = "™";

/**
 * True if `c` is an ASCII alphanumeric.  Used to decide whether
 * a `'` is an apostrophe (between letters) or a quote (after
 * whitespace).
 */
function isAlnum(c: number): boolean {
  return (
    (c >= 48 && c <= 57) ||   // 0-9
    (c >= 65 && c <= 90) ||   // A-Z
    (c >= 97 && c <= 122)     // a-z
  );
}

/**
 * True if `c` is whitespace — used for quote-direction
 * disambiguation.  Includes ASCII whitespace plus the most
 * common Unicode separators (NBSP, EM SPACE, EN SPACE).
 */
function isWhitespace(c: number): boolean {
  return (
    c === 32 || c === 9 || c === 10 || c === 13 || c === 11 || c === 12 ||
    c === 0x00A0 || c === 0x2002 || c === 0x2003 || c === 0x2009
  );
}

/**
 * Apply the configured typography substitutions to `input`.
 * Returns a new string; `input` is never mutated (strings are
 * immutable in JS anyway, but the contract is documented for
 * symmetry with the AST walker).
 *
 * Defaults: `smartQuotes`, `dashes`, `ellipsis` all `true`;
 * `ligatures` `false`.
 *
 * Complexity: O(n) in input length.  Each output character
 * costs O(1) constant work.
 */
export function typeset(input: string, options: TypographyOptions = {}): string {
  const smartQuotes = options.smartQuotes !== false;
  const dashes = options.dashes !== false;
  const ellipsis = options.ellipsis !== false;
  const ligatures = options.ligatures === true;

  // Fast path: nothing enabled → identity.
  if (!smartQuotes && !dashes && !ellipsis && !ligatures) return input;

  const s = String(input);
  const n = s.length;
  const out: string[] = [];
  // -1 sentinel means "start of string" — treated as whitespace
  // for quote-direction purposes.
  let prev = -1;

  for (let i = 0; i < n; i++) {
    const c = s.charCodeAt(i);

    // ─── Dashes: check longer pattern first ─────────────────────
    if (dashes && c === CC_HYPHEN) {
      const n1 = i + 1 < n ? s.charCodeAt(i + 1) : -1;
      const n2 = i + 2 < n ? s.charCodeAt(i + 2) : -1;
      if (n1 === CC_HYPHEN && n2 === CC_HYPHEN) {
        out.push(EM_DASH);
        prev = CC_HYPHEN;
        i += 2;
        continue;
      }
      if (n1 === CC_HYPHEN) {
        out.push(EN_DASH);
        prev = CC_HYPHEN;
        i += 1;
        continue;
      }
      // Single hyphen → passthrough.
      out.push("-");
      prev = c;
      continue;
    }

    // ─── Ellipsis: three dots ───────────────────────────────────
    if (ellipsis && c === CC_DOT) {
      const n1 = i + 1 < n ? s.charCodeAt(i + 1) : -1;
      const n2 = i + 2 < n ? s.charCodeAt(i + 2) : -1;
      if (n1 === CC_DOT && n2 === CC_DOT) {
        out.push(ELLIPSIS);
        prev = CC_DOT;
        i += 2;
        continue;
      }
      // 1-2 dots → passthrough.
      out.push(".");
      prev = c;
      continue;
    }

    // ─── Smart quotes ───────────────────────────────────────────
    if (smartQuotes && c === CC_QUOTE_DOUBLE) {
      // Left quote if at start or after whitespace; else right.
      if (prev === -1 || isWhitespace(prev)) out.push(LEFT_DQ);
      else out.push(RIGHT_DQ);
      prev = c;
      continue;
    }
    if (smartQuotes && c === CC_QUOTE_SINGLE) {
      // Apostrophe wins: between two alphanumerics → right-SQ.
      // Left-SQ only at start or after whitespace.  All other
      // contexts (after punctuation) → right-SQ (closing quote).
      if (isAlnum(prev)) out.push(RIGHT_SQ);
      else if (prev === -1 || isWhitespace(prev)) out.push(LEFT_SQ);
      else out.push(RIGHT_SQ);
      prev = c;
      continue;
    }

    // ─── Ligatures: (c) (r) (tm) ────────────────────────────────
    if (ligatures && c === CC_LPAREN) {
      const inner1 = i + 1 < n ? s.charCodeAt(i + 1) : -1;
      const inner2 = i + 2 < n ? s.charCodeAt(i + 2) : -1;
      const inner3 = i + 3 < n ? s.charCodeAt(i + 3) : -1;
      // (c) / (C)
      if ((inner1 === CC_LOWER_C || inner1 === CC_UPPER_C) && inner2 === CC_RPAREN) {
        out.push(COPYRIGHT);
        prev = CC_RPAREN;
        i += 2;
        continue;
      }
      // (r) / (R)
      if ((inner1 === CC_LOWER_R || inner1 === CC_UPPER_R) && inner2 === CC_RPAREN) {
        out.push(REGISTERED);
        prev = CC_RPAREN;
        i += 2;
        continue;
      }
      // (tm) / (TM)
      if (
        (inner1 === CC_LOWER_T || inner1 === CC_UPPER_T) &&
        (inner2 === CC_LOWER_M || inner2 === CC_UPPER_M) &&
        inner3 === CC_RPAREN
      ) {
        out.push(TRADEMARK);
        prev = CC_RPAREN;
        i += 3;
        continue;
      }
      // No match → fall through to default passthrough.
    }

    // ─── Default: passthrough ───────────────────────────────────
    out.push(s[i]!);
    prev = c;
  }

  return out.join("");
}
