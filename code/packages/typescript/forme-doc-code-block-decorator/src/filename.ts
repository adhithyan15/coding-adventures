/**
 * filename.ts — extract a `// file: foo.ts` style hint from the first
 * line of a fenced code block's content.
 *
 * =============================================================================
 * THE CONVENTION
 * =============================================================================
 *
 * In documentation prose, authors often want to show:
 *
 *     ```ts
 *     // file: src/auth.ts
 *     export function login(user) { … }
 *     ```
 *
 * — and have the renderer surface "src/auth.ts" as a small badge above
 * the code, plus *not* include that hint line in the highlighted source.
 *
 * The hint always lives on the FIRST non-blank line and uses one of
 * six comment styles, chosen to cover every language v0 cares about:
 *
 *   - `// file: …`               C, C++, Java, JS, TS, Rust, Go, Swift, …
 *   - `# file: …`                Python, Ruby, Bash, Perl, YAML, TOML, …
 *   - `-- file: …`               SQL, Haskell, Lua, Elm, …
 *   - `% file: …`                LaTeX
 *   - `<!-- file: … -->`         HTML, XML, SVG, Markdown
 *   - `/* file: … *\/`           CSS, C-block-comment fallback
 *
 * The "file:" keyword is case-insensitive (`File:`, `FILE:`, etc.).
 * Optional whitespace is allowed around the keyword and colon.  The
 * filename itself is captured as the run of non-whitespace
 * characters that follows.  Anything after the filename is ignored
 * — for HTML/C-block styles the trailing `-->` / `*\/` is part of
 * the line; for the line-comment styles, any trailing comment text
 * is permitted but discarded.
 *
 * =============================================================================
 * WHAT IF THE HINT ISN'T ON LINE 1?
 * =============================================================================
 *
 * Then it's not extracted.  Authors who want a filename badge must
 * put the hint on the first line — keeps the rule simple and
 * unambiguous.  Trailing whitespace / blank lines BEFORE the hint
 * are tolerated by the leading-newlines skip.
 *
 * @module filename
 */

// Six regexes, one per comment style.  Anchored ^…$, with `^` matching
// only the very start (no `m` flag — we test against a single line at
// a time).  Each regex captures the filename in group 1.
//
// We use [^\s]+ rather than \S+ for the filename so the regex is
// crystal-clear: "one or more non-whitespace characters."
const FILE_KW = String.raw`[Ff][Ii][Ll][Ee]\s*:\s*`;

const LINE_COMMENT_RE = new RegExp(
  String.raw`^\s*(?://|#|--|%)\s*` + FILE_KW + String.raw`([^\s]+).*$`,
);

const HTML_COMMENT_RE = new RegExp(
  String.raw`^\s*<!--\s*` + FILE_KW + String.raw`([^\s]+)\s*-->\s*$`,
);

const C_BLOCK_COMMENT_RE = new RegExp(
  String.raw`^\s*/\*\s*` + FILE_KW + String.raw`([^\s]+)\s*\*/\s*$`,
);

/**
 * Inspect a code block's raw `value` for a first-line filename hint.
 *
 * @param value - The raw code (as stored in `CodeBlockNode.value`).
 * @returns `{ filename, strippedValue }`.  When no hint is present,
 *          `filename` is `null` and `strippedValue` is the unchanged
 *          input.  When a hint is found, `filename` is the captured
 *          path and `strippedValue` is the input with the hint line
 *          (and its trailing newline) removed.
 */
export function extractFilenameHint(value: string): {
  filename: string | null;
  strippedValue: string;
} {
  // Find the first newline (handles \n and \r\n).  Everything before
  // is the candidate first line.
  const newlineIdx = value.indexOf("\n");
  let firstLine: string;
  let rest: string;
  if (newlineIdx === -1) {
    // Single-line code block — the whole value IS the first line, no
    // trailing rest.  Even if it's a filename hint, stripping leaves
    // an empty code block, which is probably not what the author
    // wants — so we still strip and let the renderer show empty.
    firstLine = value;
    rest = "";
  } else {
    firstLine = value.slice(0, newlineIdx);
    // Skip the \n itself; preserve \r when consumers want CRLF
    // (commonmark-parser already normalises to \n, so this is
    // belt-and-suspenders).
    rest = value.slice(newlineIdx + 1);
  }

  // Strip optional trailing \r from the first line (for \r\n inputs
  // that somehow reach us despite the parser normalising — defensive).
  if (firstLine.endsWith("\r")) {
    firstLine = firstLine.slice(0, -1);
  }

  // Try each pattern in turn.  HTML and C-block styles are tried
  // before the generic line-comment style because their delimiters
  // (`<!--` and `/*`) would otherwise be eaten by the leading-space
  // tolerance of the line-comment regex.
  const m =
    HTML_COMMENT_RE.exec(firstLine) ??
    C_BLOCK_COMMENT_RE.exec(firstLine) ??
    LINE_COMMENT_RE.exec(firstLine);

  if (m === null) {
    return { filename: null, strippedValue: value };
  }

  return { filename: m[1]!, strippedValue: rest };
}
