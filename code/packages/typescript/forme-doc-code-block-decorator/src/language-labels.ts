/**
 * language-labels.ts — raw language hint → display label.
 *
 * The CommonMark info-string after a fenced code block's opening
 * ``` ``` ``` carries the language as the first whitespace-delimited
 * token.  Authors typically write the SHORTEST recognisable form
 * (`ts`, `py`, `rb`, `rs`) — the parser stores that verbatim.
 *
 * A UI chrome bar showing "Copy | ts" looks unfinished.  Showing
 * "Copy | TypeScript" looks intentional.  This module is the
 * small-but-curated alias table that maps raw to display.
 *
 * Coverage matches DOC00 v0's `forme-doc-syntax-highlighter`'s
 * language set + a handful of common config / data formats authors
 * tend to drop into prose without thinking (yaml, sql, toml).  Add
 * entries as needed; unknown hints pass through verbatim.
 *
 * @module language-labels
 */

/**
 * Alias table — lowercase raw hint → display name.  Built as a
 * null-prototype object so a code block tagged ` ```__proto__ ` (or
 * `constructor`, or `toString`) can't read the inherited
 * `Object.prototype` accessor during lookup.  Defence-in-depth: the
 * fallback is "use the raw string", so a polluted lookup would just
 * emit weird-looking labels — but a `for...in` consumer enumerating
 * the result might still get surprised.  Cheap to defend.
 */
const LABELS: Record<string, string> = Object.assign(Object.create(null), {
  "ts": "TypeScript",
  "tsx": "TypeScript",
  "typescript": "TypeScript",
  "js": "JavaScript",
  "jsx": "JavaScript",
  "javascript": "JavaScript",
  "mjs": "JavaScript",
  "cjs": "JavaScript",
  "py": "Python",
  "python": "Python",
  "rb": "Ruby",
  "ruby": "Ruby",
  "go": "Go",
  "golang": "Go",
  "rs": "Rust",
  "rust": "Rust",
  "sh": "Bash",
  "bash": "Bash",
  "shell": "Bash",
  "zsh": "Bash",
  "json": "JSON",
  "html": "HTML",
  "htm": "HTML",
  "xml": "XML",
  "svg": "SVG",
  "css": "CSS",
  "scss": "SCSS",
  "sass": "Sass",
  "less": "Less",
  "md": "Markdown",
  "markdown": "Markdown",
  "yaml": "YAML",
  "yml": "YAML",
  "toml": "TOML",
  "sql": "SQL",
  "c": "C",
  "h": "C",
  "cpp": "C++",
  "cxx": "C++",
  "cc": "C++",
  "hpp": "C++",
  "java": "Java",
  "kt": "Kotlin",
  "kotlin": "Kotlin",
  "swift": "Swift",
  "scala": "Scala",
  "clj": "Clojure",
  "clojure": "Clojure",
  "ex": "Elixir",
  "exs": "Elixir",
  "elixir": "Elixir",
  "erl": "Erlang",
  "erlang": "Erlang",
  "hs": "Haskell",
  "haskell": "Haskell",
  "ml": "OCaml",
  "ocaml": "OCaml",
  "lua": "Lua",
  "pl": "Perl",
  "perl": "Perl",
  "ps1": "PowerShell",
  "powershell": "PowerShell",
  "dockerfile": "Dockerfile",
  "docker": "Dockerfile",
  "makefile": "Makefile",
  "make": "Makefile",
  "diff": "Diff",
  "patch": "Diff",
  "tex": "LaTeX",
  "latex": "LaTeX",
  "r": "R",
  "dart": "Dart",
  "zig": "Zig",
  "nim": "Nim",
});

/**
 * Map a raw language hint to its display label.
 *
 * @param raw - The raw language string from `CodeBlockNode.language`.
 *              `null` returns `null`.
 * @returns The display label.  Unknown hints fall through verbatim
 *          (preserving the author's capitalisation, since the lookup
 *          itself uses the lowercased form).
 */
export function languageLabel(raw: string | null): string | null {
  if (raw === null) return null;
  const trimmed = raw.trim();
  if (trimmed === "") return null;
  const hit = LABELS[trimmed.toLowerCase()];
  return hit ?? trimmed;
}
