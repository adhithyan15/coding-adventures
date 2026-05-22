/**
 * supported-languages.ts — informational set of language hints v1's
 * engine will recognise.
 *
 * v0 doesn't actually highlight anything (it emits one `plain` span
 * per block), so this set is consulted only by callers that want to
 * surface a "this language WILL be syntax-highlighted when v1 ships"
 * indicator in their UI / build report.  v0 itself never branches on
 * membership.
 *
 * Coverage matches DOC00 v0's syntax-highlighter spec:
 *   TypeScript, JavaScript, Python, Ruby, Go, Rust, Bash, JSON,
 *   HTML, CSS, Markdown.
 *
 * Aliases (e.g. `ts` and `typescript` both map to TypeScript) are
 * included so authors can use whichever short form they prefer in
 * their fenced-code-block info strings.
 *
 * @module supported-languages
 */

/**
 * The set of normalised (lowercased, trimmed) language hints that
 * v1's TextMate-grammar engine will recognise as "yes, I have a
 * grammar for this".  Static / immutable — exposed as a frozen Set
 * so downstream consumers can iterate but can't mutate.
 */
export const SUPPORTED_LANGUAGES: ReadonlySet<string> = new Set([
  // TypeScript
  "ts", "tsx", "typescript",
  // JavaScript
  "js", "jsx", "javascript", "mjs", "cjs",
  // Python
  "py", "python",
  // Ruby
  "rb", "ruby",
  // Go
  "go", "golang",
  // Rust
  "rs", "rust",
  // Bash
  "sh", "bash", "shell", "zsh",
  // JSON
  "json",
  // HTML
  "html", "htm",
  // CSS
  "css",
  // Markdown
  "md", "markdown",
]);

/**
 * Check whether `language` is on the v1 highlighter's supported list.
 * Pure helper for callers building "would be highlighted" badges.
 *
 * @param language - The raw language string (case-insensitive,
 *                   whitespace-trimmed).  `null` returns `false`.
 * @returns `true` iff `language` (or a recognised alias) will be
 *          highlighted by v1.
 */
export function isSupportedLanguage(language: string | null): boolean {
  if (language === null) return false;
  return SUPPORTED_LANGUAGES.has(language.trim().toLowerCase());
}
