/**
 * escape.ts — LaTeX special-character escaping.
 *
 * LaTeX has ten characters that trigger non-literal behaviour in text
 * mode (and a few more in math mode, but we never emit math mode
 * from here):
 *
 *   \  command introducer
 *   %  comment-to-end-of-line
 *   $  toggle math mode
 *   &  alignment tab
 *   _  subscript (math) / fragile (text)
 *   #  macro parameter
 *   {  group open
 *   }  group close
 *   ^  superscript (math) / accent (text)
 *   ~  non-breaking space
 *
 * Any of these landing unescaped in our output can:
 *
 *   - silently swallow output up to the next newline (`%`),
 *   - drop the LaTeX compiler into math mode (`$`),
 *   - terminate a group early or open one (`{` / `}`),
 *   - invoke an unintended command (`\foo` where `foo` is anything),
 *   - or trigger a compile error that points at our generated file
 *     rather than the offending source.
 *
 * We escape every special character in any string we interpolate
 * into the output — color names, font names, selector targets,
 * id strings, role strings.  Real-world input is clean ASCII; the
 * escaper is defence in depth for hand-rolled IRs that bypass the
 * forme-style-ir validator's identifier grammar.
 *
 * ASCII control characters (0x00–0x1F and 0x7F) are stripped
 * outright — they can't appear in a legitimate selector / identifier
 * and they confuse LaTeX's lexer.
 *
 * @module escape
 */

/**
 * Escape LaTeX special characters in a free-form string destined for
 * text-mode output (e.g. inside a `\textbf{...}` body or a comment).
 *
 * **Order matters.**  Naively escaping `\` first would produce
 * `\textbackslash{}` — but the `{` / `}` inside that escape would
 * then get re-escaped on subsequent passes, yielding the broken
 * `\textbackslash\{\}`.  The fix: substitute a unique
 * never-appears-in-input placeholder for each multi-character escape
 * during the pass, then swap the placeholders back at the end.
 *
 * The placeholder strings (`\x00BS`, `\x00CARET`, `\x00TILDE`) start
 * with NUL bytes that `stripControl` has already removed from the
 * input — so they cannot collide with anything the user supplied.
 */
const BACKSLASH_PLACEHOLDER = "\x00BS\x00";
const CARET_PLACEHOLDER     = "\x00CARET\x00";
const TILDE_PLACEHOLDER     = "\x00TILDE\x00";

export function escapeLatexText(s: string): string {
  return stripControl(s)
    // First pass — replace multi-character escape targets with
    // placeholders so their literal `{` / `}` don't get
    // double-escaped on the brace pass below.
    .replace(/\\/g, BACKSLASH_PLACEHOLDER)
    .replace(/\^/g, CARET_PLACEHOLDER)
    .replace(/~/g, TILDE_PLACEHOLDER)
    // Single-char escapes — safe to do in any order at this point.
    .replace(/%/g, "\\%")
    .replace(/\$/g, "\\$")
    .replace(/&/g, "\\&")
    .replace(/_/g, "\\_")
    .replace(/#/g, "\\#")
    .replace(/\{/g, "\\{")
    .replace(/\}/g, "\\}")
    // Swap placeholders back to their LaTeX-correct escapes.
    .replace(new RegExp(BACKSLASH_PLACEHOLDER, "g"), "\\textbackslash{}")
    .replace(new RegExp(CARET_PLACEHOLDER, "g"), "\\textasciicircum{}")
    .replace(new RegExp(TILDE_PLACEHOLDER, "g"), "\\textasciitilde{}");
}

/**
 * Sanitise a string for use as a LaTeX identifier (e.g. a color
 * name fed to `\definecolor{<name>}{...}{...}`).  LaTeX command
 * names are restricted to letters only (no digits, no punctuation,
 * no Unicode) so we map any non-letter to `Z<hex>Z` — a reversible
 * encoding that's still a valid letter run.
 *
 * The output is **always** a non-empty string of ASCII letters.  An
 * empty input is rejected as `Zempty` (defensive; the validator
 * doesn't admit empty token names).
 */
export function latexIdent(s: string): string {
  const sanitised = stripControl(s);
  if (sanitised.length === 0) return "Zempty";
  let out = "";
  for (const ch of sanitised) {
    if (/[A-Za-z]/.test(ch)) {
      out += ch;
    } else {
      const code = ch.codePointAt(0)!;
      out += `Z${code.toString(16)}Z`;
    }
  }
  // Refuse to emit an empty identifier (theoretical — sanitised
  // is non-empty and every char becomes at least one character).
  return out.length === 0 ? "Zempty" : out;
}

/**
 * Strip ASCII control characters (0x00–0x1F and 0x7F).  These are
 * never legitimate in a selector / identifier / token name and they
 * confuse LaTeX's lexer.
 */
function stripControl(s: string): string {
  // eslint-disable-next-line no-control-regex
  return s.replace(/[\x00-\x1F\x7F]/g, "");
}
