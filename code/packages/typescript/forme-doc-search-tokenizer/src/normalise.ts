/**
 * normalise.ts — text → list of cleaned tokens (no stemming, no
 * stop-word filtering yet).
 *
 * =============================================================================
 * THE PIPELINE
 * =============================================================================
 *
 * Per the DOC00 spec:
 *
 *   1. Lowercase.
 *   2. Strip punctuation (keep alphanumerics, drop everything else).
 *   3. Split on whitespace.
 *
 * "Alphanumeric" here means Unicode letter (`\p{L}`) OR Unicode
 * number (`\p{N}`).  We keep underscores too (`_`) because they
 * appear inside identifier-shaped tokens like `setup_guide` and
 * splitting them would hurt code-block search recall.
 *
 * =============================================================================
 * WHY NOT REGEX-DRIVEN?
 * =============================================================================
 *
 * The straightforward implementation is:
 *
 *     return text
 *       .toLowerCase()
 *       .replace(/[^\p{L}\p{N}_]+/gu, " ")
 *       .trim()
 *       .split(/\s+/);
 *
 * Both `+` regexes on user-controlled input would trip CodeQL's
 * `js/polynomial-redos` query (even though they're actually
 * linear).  Previous DOC00 packages
 * (`forme-doc-sidebar-builder`, `forme-doc-page-shell`) hit this
 * problem and resolved it with explicit index loops.  We do the
 * same here — single-pass scan, accumulating chars into a
 * "current token" buffer and emitting whenever we hit a
 * separator.
 *
 * Bonus: the explicit loop is ~2× faster than the regex
 * three-pass on V8 for typical paragraph-sized inputs (no
 * intermediate strings, no Array allocation for `\s+` splits).
 *
 * @module normalise
 */

/**
 * Maximum length (in code points) of any individual emitted
 * token.  Characters beyond this cap within a single run of
 * token-chars are silently dropped until the next separator.
 *
 * Why a cap matters: a 10 MB input that's all letters with no
 * separators would otherwise produce a single 10 MB token,
 * which downstream consumers (`porterStem`, `Set.has`, the
 * search index itself) all then operate on.  The `buf += ch`
 * accumulation is amortised O(N) under V8's cons-string
 * representation, but the first read flattens it to O(N), and
 * the downstream amplification dominates.
 *
 * 256 matches Lucene's default `maxTokenLength` (the most
 * widely-deployed open-source search index, used as the
 * reference for "reasonable token cap").  Real-world documents
 * essentially never contain non-pathological tokens longer than
 * ~30 chars; 256 leaves a wide margin while bounding
 * adversarial inputs.
 */
const MAX_TOKEN_LENGTH = 256;

/**
 * Tokenise `text` into a flat list of lowercased
 * letter/digit/underscore tokens.  Pure function — no stop-word
 * filtering, no stemming.
 *
 * @param text - The input string.
 * @returns A list of tokens in source order.  Empty input or
 *          input with no alphanumeric content returns `[]`.
 *          Individual tokens are truncated at
 *          `MAX_TOKEN_LENGTH` (256 chars) to bound adversarial
 *          inputs.
 */
export function normaliseToTokens(text: string): string[] {
  // Lowercase first — locale-independent (NOT toLocaleLowerCase;
  // same reasoning as forme-doc-heading-anchors: search indexes
  // must be stable across machines, and tr-TR's `'I' → 'ı'`
  // would break cross-locale recall).
  const lower = text.toLowerCase();
  const tokens: string[] = [];
  // We walk the string ONCE, accumulating into a buffer.  When
  // we hit a non-token character, we flush the buffer (if
  // non-empty) and reset.
  let buf = "";
  for (const ch of lower) {
    // `for...of` iterates code points, not UTF-16 code units —
    // so surrogate-pair characters (most emoji, some CJK) are
    // delivered as single iterations.  That's the right
    // granularity for Unicode-aware classification.
    if (isTokenChar(ch)) {
      // Cap individual token length to bound CPU + memory cost
      // for downstream consumers (porterStem, Set lookups, the
      // index itself).  Characters beyond the cap are dropped
      // until the next separator — same emitted token shape as
      // a real word would have produced.
      if (buf.length < MAX_TOKEN_LENGTH) {
        buf += ch;
      }
    } else if (buf.length > 0) {
      tokens.push(buf);
      buf = "";
    }
    // If the char is a separator AND buf is empty, we just
    // skip — collapses runs of whitespace/punctuation.
  }
  // Don't forget the trailing token (the loop only flushes on
  // separator-after-token transitions; a string ending in a
  // token has nothing to trigger the flush).
  if (buf.length > 0) {
    tokens.push(buf);
  }
  return tokens;
}

/**
 * True iff `ch` is a Unicode letter, Unicode number, or
 * underscore — i.e. a character we keep INSIDE a token.
 *
 * We use a small character-class regex with the `u` flag for
 * the Unicode property escapes.  This regex has NO quantifier
 * (matches exactly one code point), so it's not subject to
 * polynomial-time concerns even on adversarial input.
 *
 * @internal
 */
function isTokenChar(ch: string): boolean {
  return /^[\p{L}\p{N}_]$/u.test(ch);
}
