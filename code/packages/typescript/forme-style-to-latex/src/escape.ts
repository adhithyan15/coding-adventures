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
 * Implementation note: **one single-pass replacement** — not a chain.
 * A chained-`.replace` approach (escape `\` first, then `%`, then
 * `&`, …) produces wrong output because the synthetic
 * `\textbackslash{}` introduced for `\` contains `{` and `}`, which
 * the later brace-escape passes then re-escape into
 * `\textbackslash\{\}`.  A single pass over the original string
 * with a per-character mapping table sidesteps this entirely — and
 * is the form CodeQL's "incomplete string escaping" rule accepts
 * without complaint.
 */
const LATEX_ESCAPE_MAP: Readonly<Record<string, string>> = Object.freeze({
  "\\": "\\textbackslash{}",
  "%":  "\\%",
  "$":  "\\$",
  "&":  "\\&",
  "_":  "\\_",
  "#":  "\\#",
  "{":  "\\{",
  "}":  "\\}",
  "^":  "\\textasciicircum{}",
  "~":  "\\textasciitilde{}",
});

// Class of LaTeX-special characters.  Order inside `[]` doesn't
// matter for matching; the `]` before the character class form
// (`[\\%$&_#{}^~]`) is just a single pass.
const LATEX_SPECIAL_RE = /[\\%$&_#{}^~]/g;

export function escapeLatexText(s: string): string {
  return stripControl(s).replace(LATEX_SPECIAL_RE, (ch) => LATEX_ESCAPE_MAP[ch]!);
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
